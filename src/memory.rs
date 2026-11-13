//! Durable branch context projected from canonical records, never from transient chat.
use crate::domain::*;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize)]
pub struct BranchMemory {
    pub selected: RecordId,
    pub question: Option<QuestionId>,
    pub records: Vec<MemoryRecord>,
    pub relationships: Vec<Edge>,
    pub omitted_records: Vec<RecordId>,
    pub truncated_text: Vec<RecordId>,
    pub notice: &'static str,
}
#[derive(Clone, Debug, Serialize)]
pub struct MemoryRecord {
    pub id: RecordId,
    pub role: &'static str,
    pub title: String,
    pub question_id: Option<QuestionId>,
    pub insight: HarnessInsight,
    pub evidence: Vec<ObservationId>,
    pub counter_evidence: Vec<ObservationId>,
    pub revises: Option<RecordId>,
    pub type_candidate: Option<String>,
    pub plan: Option<QuestionPlan>,
    pub note: String,
}

pub fn clip(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        text.into()
    } else {
        format!(
            "{} [excerpt; retained record has more]",
            text.chars().take(limit).collect::<String>()
        )
    }
}

/// AI follow-up questions keep their recorded initiating branch. Analyst reviews inherit
/// their original target's branch. Both links are data already retained in the case.
pub fn branch_question(session: &Investigation, id: RecordId) -> Option<QuestionId> {
    let mut current = id;
    let mut seen = HashSet::new();
    while seen.insert(current) {
        let record = session.record(current)?;
        if let Some(run) = session
            .ai_runs
            .iter()
            .find(|r| r.record == current || r.analysis_record == Some(current))
            && let Some(parent) = run.context["selected_record"].as_u64()
            && parent != current.0
            && session.record(RecordId(parent)).is_some()
        {
            current = RecordId(parent);
            continue;
        }
        if let RecordData::Review { target, .. } = record.data {
            current = target;
            continue;
        }
        if let Some(owner) = record.question_id {
            current = owner.record();
            continue;
        }
        return QuestionId::new(current.0)
            .ok()
            .filter(|q| q.resolve(session).is_ok());
    }
    None
}
pub fn references(record: &Record) -> (Vec<ObservationId>, Vec<ObservationId>, Option<RecordId>) {
    match &record.data {
        RecordData::Explanation { evidence, .. } => (evidence.clone(), vec![], None),
        RecordData::TypeHypothesis(h) => (h.evidence.clone(), h.counter_evidence.clone(), None),
        RecordData::Review {
            target, evidence, ..
        } => (evidence.clone(), vec![], Some(*target)),
        _ => (vec![], vec![], None),
    }
}
fn project(record: &Record) -> (MemoryRecord, bool) {
    let mut shortened = false;
    let mut text = |s: &str, limit| {
        let result = clip(s, limit);
        shortened |= result != s;
        result
    };
    let title = text(&record.title, 180);
    let insight = HarnessInsight {
        question: text(&record.insight.question, 500),
        purpose: text(&record.insight.purpose, 500),
        action: text(&record.insight.action, 500),
        observation: text(&record.insight.observation, 900),
        explanation: text(&record.insight.explanation, 900),
        uncertainty: text(&record.insight.uncertainty, 900),
        next_step: text(&record.insight.next_step, 500),
    };
    let note = text(&record.note, 900);
    let plan = record.question_plan.as_ref().map(|p| QuestionPlan {
        why: text(&p.why, 500),
        steps: p.steps.iter().take(8).map(|s| text(s, 400)).collect(),
        enables: text(&p.enables, 500),
        answered_when: text(&p.answered_when, 500),
        if_not: text(&p.if_not, 500),
    });
    if record
        .question_plan
        .as_ref()
        .is_some_and(|p| p.steps.len() > 8)
    {
        shortened = true;
    }
    let (evidence, counter_evidence, revises) = references(record);
    let role = match &record.data {
        RecordData::Explanation {
            basis: Basis::Inferred,
            ..
        } => "inferred interpretation",
        RecordData::Explanation { .. } => "supported interpretation",
        RecordData::TypeHypothesis(_) => "hypothesis; not an observation",
        RecordData::Review {
            kind: crate::history::ReviewKind::Correction,
            ..
        } => "analyst correction",
        RecordData::Review {
            kind: crate::history::ReviewKind::Resolution,
            ..
        } => "scoped analyst resolution",
        RecordData::Review { .. } => "analyst steering",
        other => other.label(),
    };
    (
        MemoryRecord {
            id: record.id,
            role,
            title,
            question_id: record.question_id,
            insight,
            evidence,
            counter_evidence,
            revises,
            type_candidate: match &record.data {
                RecordData::TypeHypothesis(h) => Some(clip(&h.proposed_type, 300)),
                _ => None,
            },
            plan,
            note,
        },
        shortened,
    )
}

pub fn branch(
    session: &Investigation,
    visible: usize,
    selected: RecordId,
) -> Result<BranchMemory, String> {
    let available = &session.records[..visible.min(session.records.len())];
    if !available.iter().any(|r| r.id == selected) {
        return Err("Selected branch is beyond the evidence frontier".into());
    }
    let question = branch_question(session, selected);
    let mut priority = vec![selected];
    if let Some(q) = question {
        priority.push(q.record());
    }
    // Explicit decisions and hypotheses precede ordinary recent records.
    for decisions in [true, false] {
        for record in available.iter().rev() {
            if question.is_some()
                && branch_question(session, record.id) == question
                && matches!(
                    record.data,
                    RecordData::Review { .. } | RecordData::TypeHypothesis(_)
                ) == decisions
            {
                priority.push(record.id);
            }
        }
    }
    // Include adjacent questions and the original findings/evidence referenced by the branch.
    let roots: HashSet<_> = priority.iter().copied().collect();
    for edge in &session.edges {
        if roots.contains(&edge.from) {
            priority.push(edge.to);
        }
        if roots.contains(&edge.to) {
            priority.push(edge.from);
        }
    }
    let mut ordered = vec![];
    let mut seen = HashSet::new();
    // Preserve each prioritized record together with its evidence/counter-evidence dependencies.
    fn add(
        id: RecordId,
        available: &[Record],
        seen: &mut HashSet<RecordId>,
        output: &mut Vec<RecordId>,
    ) {
        if !seen.insert(id) {
            return;
        }
        let Some(record) = available.iter().find(|r| r.id == id) else {
            return;
        };
        output.push(id);
        let (evidence, counter, target) = references(record);
        for id in evidence
            .iter()
            .chain(&counter)
            .map(|id| id.record())
            .chain(target)
        {
            add(id, available, seen, output);
        }
    }
    for id in priority {
        add(id, available, &mut seen, &mut ordered);
    }
    let mut memory = BranchMemory {
        selected,
        question,
        records: vec![],
        relationships: vec![],
        omitted_records: vec![],
        truncated_text: vec![],
        notice: "Projection of retained case records. Inferences and analyst decisions are not observations. Omitted records/text remain in the case; do not assume this snapshot is complete.",
    };
    let mut bytes = 0;
    for id in ordered {
        let (record, shortened) = project(session.record(id).unwrap());
        let size = serde_json::to_vec(&record).unwrap().len();
        if bytes + size <= 30_000 && memory.records.len() < 24 {
            bytes += size;
            if shortened {
                memory.truncated_text.push(id);
            }
            memory.records.push(record);
        } else {
            memory.omitted_records.push(id);
        }
    }
    let ids: HashSet<_> = memory.records.iter().map(|r| r.id).collect();
    memory.relationships = session
        .edges
        .iter()
        .filter(|e| ids.contains(&e.from) && ids.contains(&e.to))
        .cloned()
        .collect();
    Ok(memory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        evidence::TypeHypothesis,
        history::{History, ReviewKind},
    };
    #[test]
    fn branch_memory_survives_replay_and_retains_counter_evidence_and_corrections() {
        let mut session = Investigation::workshop();
        let mut history = History::new(&session, "Synthetic memory fixture").unwrap();
        let original = serde_json::to_value(&session.records[3]).unwrap();
        let index = session
            .append_type_hypothesis(TypeHypothesis {
                proposed_type: "u16 candidate".into(),
                evidence: vec![ObservationId::new(3).unwrap()],
                counter_evidence: vec![ObservationId::new(3).unwrap()],
                rationale: "Two bytes permit a u16 view".into(),
                limitation: "The same bytes also permit separate u8 values".into(),
            })
            .unwrap();
        let hypothesis = session.records[index].id;
        session
            .append_review(
                ReviewKind::Correction,
                hypothesis,
                "Keep byte order hypothetical".into(),
                "No consumer observed".into(),
                vec![ObservationId::new(3).unwrap()],
            )
            .unwrap();
        history.capture(&session).unwrap();
        let before = branch(&session, session.records.len(), hypothesis).unwrap();
        assert!(
            before
                .records
                .iter()
                .any(|r| r.role == "analyst correction")
        );
        assert!(
            before
                .records
                .iter()
                .any(|r| r.counter_evidence == [ObservationId::new(3).unwrap()])
        );
        assert!(before.records.iter().any(|r| r.id == RecordId(3)));
        assert_eq!(original, serde_json::to_value(&session.records[3]).unwrap());
        let replay = history.replay(history.events().len()).unwrap();
        assert_eq!(
            serde_json::to_value(before).unwrap(),
            serde_json::to_value(branch(&replay, replay.records.len(), hypothesis).unwrap())
                .unwrap()
        );
        assert!(branch(&session, 3, hypothesis).is_err());
    }
    #[test]
    fn old_branch_evidence_is_not_displaced_by_unrelated_recent_results() {
        let mut session = Investigation::workshop();
        for id in 7..47 {
            let mut record = session.records[2].clone();
            record.id = RecordId(id);
            record.question_id = Some(QuestionId::new(5).unwrap());
            record.title = format!("Other branch observation {id}");
            session.records.push(record);
        }
        session.validate().unwrap();
        let context = crate::omp::context(&session, session.records.len(), RecordId(1)).unwrap();
        assert!(
            context["record_ids"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(3))
        );
        assert!(
            !context["branch_memory"]["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == 46)
        );
        let correction = session
            .append_review(
                ReviewKind::Correction,
                RecordId(4),
                "Do not confuse the chosen integer view with intended meaning".into(),
                "Consumer still unknown".into(),
                vec![ObservationId::new(3).unwrap()],
            )
            .unwrap();
        let context = crate::omp::context(&session, session.records.len(), RecordId(1)).unwrap();
        assert!(
            context["branch_memory"]["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == session.records[correction].id.0)
        );
        let old = crate::omp::context(&session, 6, RecordId(1)).unwrap();
        assert!(!old.to_string().contains("Consumer still unknown"));
    }
    #[test]
    fn large_memory_marks_omissions_and_excerpts_without_mutating_the_case() {
        let mut session = Investigation::workshop();
        for _ in 0..30 {
            session
                .append_review(
                    ReviewKind::Steering,
                    RecordId(1),
                    "Long teaching rationale. ".repeat(100),
                    "Evidence remains limited. ".repeat(100),
                    vec![],
                )
                .unwrap();
        }
        let before = serde_json::to_value(&session).unwrap();
        let memory = branch(&session, session.records.len(), RecordId(1)).unwrap();
        assert!(!memory.omitted_records.is_empty());
        assert!(!memory.truncated_text.is_empty());
        let context = crate::omp::context(&session, session.records.len(), RecordId(1)).unwrap();
        assert!(serde_json::to_vec(&context).unwrap().len() < 64000);
        assert_eq!(serde_json::to_value(&session).unwrap(), before);
    }
}
