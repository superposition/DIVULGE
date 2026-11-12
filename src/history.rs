//! Ordered, append-only case history. Replay is pure data reconstruction.
use crate::domain::{Investigation, Record, RecordData, RecordId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FORMAT: &str = "divulge-event-ledger";
pub const VERSION: u32 = 1;
const MAX_EVENTS: usize = 4096;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub format: String,
    pub version: u32,
    pub baseline_provenance: String,
    pub baseline: Investigation,
    pub events: Vec<Event>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub sequence: u64,
    pub previous_sha256: String,
    pub payload: Payload,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Payload {
    Commit(Vec<Change>),
    JobStarted { id: String, adapter: String },
    JobEnded { id: String, outcome: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Change {
    RecordAdded {
        role: Role,
        record: Box<Record>,
    },
    EdgesAdded(Vec<crate::domain::Edge>),
    Disassembly(Box<crate::pi32::DisassemblyReport>),
    ModelReceipt(Box<crate::omp::AiRun>),
    ReaReceipt(Box<crate::rea::ReaRun>),
    NoteRevision {
        record: RecordId,
        previous: String,
        text: String,
    },
    ObjectiveSteered {
        previous: String,
        text: String,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Question,
    Action,
    Observation,
    Explanation,
    Hypothesis,
    OpenQuestion,
    Steering,
    Revision,
    Resolution,
}
impl Role {
    fn of(record: &Record) -> Self {
        match record.data {
            RecordData::Question => Self::Question,
            RecordData::Probe { .. } => Self::Action,
            RecordData::Observation(_)
            | RecordData::Firmware(_)
            | RecordData::Disassembly(_)
            | RecordData::Rea(_) => Self::Observation,
            RecordData::Explanation { .. } => Self::Explanation,
            RecordData::TypeHypothesis(_) => Self::Hypothesis,
            RecordData::Unknown { .. } => Self::OpenQuestion,
            RecordData::Review { kind, .. } => match kind {
                ReviewKind::Steering => Self::Steering,
                ReviewKind::Correction => Self::Revision,
                ReviewKind::Resolution => Self::Resolution,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewKind {
    #[default]
    Steering,
    Correction,
    Resolution,
}

impl Investigation {
    pub fn append_review(
        &mut self,
        kind: ReviewKind,
        target: RecordId,
        rationale: String,
        limitation: String,
        evidence: Vec<crate::domain::ObservationId>,
    ) -> Result<usize, String> {
        use crate::domain::{Edge, HarnessInsight, Relation};
        if self.records.len() >= 200
            || rationale.trim().is_empty()
            || limitation.trim().is_empty()
            || rationale.len() > 4000
            || limitation.len() > 4000
        {
            return Err("Review requires bounded rationale and limitations".into());
        }
        let prior = self.record(target).ok_or("Review target is missing")?;
        let question = prior.insight.question.clone();
        let question_id = self
            .question_for(target)
            .map(|r| crate::domain::QuestionId::new(r.id.0))
            .transpose()?;
        let id = RecordId(
            self.records
                .iter()
                .map(|r| r.id.0)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or("Record ID overflow")?,
        );
        let count = self.records.len();
        let edges = self.edges.len();
        self.records.push(Record {
            id,
            title: format!("{kind:?} / record #{}", target.0),
            summary: rationale.chars().take(160).collect(),
            data: RecordData::Review {
                kind,
                target,
                evidence,
            },
            note: String::new(),
            question_id,
            question_plan: None,
            insight: HarnessInsight {
                question,
                purpose: "Preserve an explicit analyst decision and its scope".into(),
                action: format!(
                    "Analyst recorded {kind:?}; the original finding remains unchanged"
                ),
                observation: "This is an analyst decision, not a new tool observation".into(),
                explanation: rationale,
                uncertainty: limitation,
                next_step: "Review the cited evidence and the original record together".into(),
            },
        });
        self.edges.push(Edge {
            from: target,
            to: id,
            relation: match kind {
                ReviewKind::Steering => Relation::Steers,
                ReviewKind::Correction => Relation::Revises,
                ReviewKind::Resolution => Relation::Resolves,
            },
        });
        if let Err(error) = self.validate() {
            self.records.truncate(count);
            self.edges.truncate(edges);
            return Err(error);
        }
        Ok(count)
    }
}

fn equal<T: Serialize>(a: &T, b: &T) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}
fn digest<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(crate::fwsc::digest(
        &serde_json::to_vec(value).map_err(|e| e.to_string())?,
    ))
}
fn prefix<'a, T: Serialize>(old: &[T], new: &'a [T]) -> Result<&'a [T], String> {
    if new.len() < old.len() || !old.iter().zip(new).all(|(a, b)| equal(a, b)) {
        return Err("History cannot remove or silently rewrite retained evidence".into());
    }
    Ok(&new[old.len()..])
}

/// Cached projection is never serialized; loading always reconstructs it.
#[derive(Clone, Debug)]
pub struct History {
    document: Document,
    current: Investigation,
    active_jobs: BTreeMap<String, String>,
}
impl History {
    pub fn new(session: &Investigation, provenance: &str) -> Result<Self, String> {
        session.validate()?;
        if provenance.is_empty() || provenance.len() > 1000 {
            return Err("Invalid baseline provenance".into());
        }
        Ok(Self {
            document: Document {
                format: FORMAT.into(),
                version: VERSION,
                baseline_provenance: provenance.into(),
                baseline: session.clone(),
                events: vec![],
            },
            current: session.clone(),
            active_jobs: BTreeMap::new(),
        })
    }
    pub fn restore(document: Document) -> Result<Self, String> {
        if document.format != FORMAT
            || document.version != VERSION
            || document.events.len() > MAX_EVENTS
        {
            return Err("Unsupported or oversized event ledger".into());
        }
        let mut history = Self::new(&document.baseline, &document.baseline_provenance)?;
        for event in document.events {
            if event.sequence != history.document.events.len() as u64 + 1
                || event.previous_sha256 != history.head()?
                || event.sha256
                    != digest(&(event.sequence, &event.previous_sha256, &event.payload))?
            {
                return Err("Event order or content commitment mismatch".into());
            }
            history.apply(&event.payload)?;
            history.document.events.push(event);
        }
        Ok(history)
    }
    fn head(&self) -> Result<String, String> {
        self.document.events.last().map_or_else(
            || digest(&(&self.document.baseline_provenance, &self.document.baseline)),
            |e| Ok(e.sha256.clone()),
        )
    }
    fn append(&mut self, payload: Payload) -> Result<(), String> {
        if self.document.events.len() >= MAX_EVENTS {
            return Err("Event ledger limit reached; save and begin a new case".into());
        }
        let sequence = self.document.events.len() as u64 + 1;
        let previous_sha256 = self.head()?;
        let sha256 = digest(&(sequence, &previous_sha256, &payload))?;
        self.apply(&payload)?;
        self.document.events.push(Event {
            sequence,
            previous_sha256,
            payload,
            sha256,
        });
        Ok(())
    }
    fn apply(&mut self, payload: &Payload) -> Result<(), String> {
        match payload {
            Payload::JobStarted { id, adapter } => {
                if id.is_empty()
                    || id.len() > 200
                    || adapter.is_empty()
                    || adapter.len() > 100
                    || self
                        .document
                        .events
                        .iter()
                        .any(|e| matches!(&e.payload, Payload::JobStarted{id:old,..} if old==id))
                {
                    return Err("Invalid or repeated job identity".into());
                }
                self.active_jobs.insert(id.clone(), adapter.clone());
            }
            Payload::JobEnded { id, outcome } => {
                if outcome.is_empty() || outcome.len() > 4000 || !self.active_jobs.contains_key(id)
                {
                    return Err("Job outcome has no active start".into());
                }
                self.active_jobs.remove(id);
            }
            Payload::Commit(changes) => {
                if changes.is_empty() || changes.len() > 1000 {
                    return Err("Invalid change transaction".into());
                }
                let mut next = self.current.clone();
                for change in changes {
                    match change {
                        Change::RecordAdded { role, record } => {
                            if *role != Role::of(record)
                                || next.records.iter().any(|r| r.id == record.id)
                            {
                                return Err("Invalid event record role or duplicate ID".into());
                            }
                            next.records.push(*record.clone());
                        }
                        Change::EdgesAdded(edges) => next.edges.extend(edges.clone()),
                        Change::Disassembly(run) => next.disassemblies.push(*run.clone()),
                        Change::ModelReceipt(run) => next.ai_runs.push(*run.clone()),
                        Change::ReaReceipt(run) => next.rea_runs.push(*run.clone()),
                        Change::NoteRevision {
                            record,
                            previous,
                            text,
                        } => {
                            let r = next
                                .records
                                .iter_mut()
                                .find(|r| r.id == *record)
                                .ok_or("Note revision references a missing record")?;
                            if &r.note != previous || text.len() > 16000 {
                                return Err("Note revision does not follow retained text".into());
                            }
                            r.note = text.clone();
                        }
                        Change::ObjectiveSteered { previous, text } => {
                            if &next.objective != previous
                                || text.trim().is_empty()
                                || text.len() > 4000
                            {
                                return Err("Invalid objective steering".into());
                            }
                            next.objective = text.clone();
                        }
                    }
                }
                next.validate()?;
                self.current = next;
            }
        }
        Ok(())
    }
    /// One atomic transaction; immutable records cannot be rewritten through a save.
    pub fn capture(&mut self, next: &Investigation) -> Result<bool, String> {
        next.validate()?;
        let old = &self.current;
        if old.schema_version != next.schema_version
            || old.title != next.title
            || old.origin != next.origin
            || !equal(&old.artifact, &next.artifact)
        {
            return Err("A different target or baseline requires a separate investigation".into());
        }
        if next.records.len() < old.records.len() {
            return Err("Cannot remove historical records".into());
        }
        let mut changes = vec![];
        for (before, after) in old.records.iter().zip(&next.records) {
            let mut normalized = after.clone();
            normalized.note = before.note.clone();
            if !equal(before, &normalized) {
                return Err("Existing findings are immutable; append a correction instead".into());
            }
            if before.note != after.note {
                changes.push(Change::NoteRevision {
                    record: before.id,
                    previous: before.note.clone(),
                    text: after.note.clone(),
                });
            }
        }
        if old.objective != next.objective {
            changes.push(Change::ObjectiveSteered {
                previous: old.objective.clone(),
                text: next.objective.clone(),
            });
        }
        for record in &next.records[old.records.len()..] {
            changes.push(Change::RecordAdded {
                role: Role::of(record),
                record: Box::new(record.clone()),
            });
        }
        let edges = prefix(&old.edges, &next.edges)?;
        if !edges.is_empty() {
            changes.push(Change::EdgesAdded(edges.to_vec()));
        }
        for run in prefix(&old.disassemblies, &next.disassemblies)? {
            changes.push(Change::Disassembly(Box::new(run.clone())));
        }
        for run in prefix(&old.ai_runs, &next.ai_runs)? {
            changes.push(Change::ModelReceipt(Box::new(run.clone())));
        }
        for run in prefix(&old.rea_runs, &next.rea_runs)? {
            changes.push(Change::ReaReceipt(Box::new(run.clone())));
        }
        if changes.is_empty() {
            return Ok(false);
        }
        self.append(Payload::Commit(changes))?;
        Ok(true)
    }
    pub fn start_job(&mut self, id: &str, adapter: &str) -> Result<(), String> {
        self.append(Payload::JobStarted {
            id: id.into(),
            adapter: adapter.into(),
        })
    }
    pub fn finish_job(&mut self, id: &str, outcome: &str) -> Result<(), String> {
        self.append(Payload::JobEnded {
            id: id.into(),
            outcome: outcome.into(),
        })
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn session(&self) -> &Investigation {
        &self.current
    }
    pub fn events(&self) -> &[Event] {
        &self.document.events
    }
    pub fn unfinished_jobs(&self) -> &BTreeMap<String, String> {
        &self.active_jobs
    }
    /// Prefix replay has no process, filesystem, credentials or network capability.
    pub fn replay(&self, through: usize) -> Result<Investigation, String> {
        if through > self.document.events.len() {
            return Err("Replay frontier is outside history".into());
        }
        let mut document = self.document.clone();
        document.events.truncate(through);
        Ok(Self::restore(document)?.current)
    }
}

impl Event {
    pub fn summary(&self) -> String {
        match &self.payload {
            Payload::JobStarted { id, adapter } => format!("{adapter} started / {id}"),
            Payload::JobEnded { id, outcome } => format!("{id} / {outcome}"),
            Payload::Commit(changes) => changes
                .iter()
                .map(|c| match c {
                    Change::RecordAdded { role, record } => format!("{role:?} #{}", record.id.0),
                    Change::NoteRevision { record, .. } => format!("Note revision #{}", record.0),
                    Change::ObjectiveSteered { .. } => "Objective steering".into(),
                    Change::EdgesAdded(v) => format!("{} relationships", v.len()),
                    Change::Disassembly(_) => "Disassembly receipt".into(),
                    Change::ModelReceipt(_) => "Model receipt".into(),
                    Change::ReaReceipt(_) => "REA receipt".into(),
                })
                .collect::<Vec<_>>()
                .join(" / "),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_retains_revisions_and_rejects_history_rewrites() {
        let mut session = Investigation::workshop();
        let mut history = History::new(&session, "Synthetic baseline").unwrap();
        history.start_job("synthetic-1", "fixture").unwrap();
        session.records[2].note = "First prediction".into();
        history.capture(&session).unwrap();
        session.records[2].note = "Revised prediction".into();
        history.capture(&session).unwrap();
        history.finish_job("synthetic-1", "cancelled").unwrap();
        let bytes = serde_json::to_vec(history.document()).unwrap();
        let restored = History::restore(serde_json::from_slice(&bytes).unwrap()).unwrap();
        assert!(equal(restored.session(), &session));
        assert_eq!(
            restored.replay(2).unwrap().records[2].note,
            "First prediction"
        );
        assert!(restored.replay(1).unwrap().records[2].note.is_empty());
        session.records[2].summary = "silently rewritten".into();
        assert!(history.capture(&session).is_err());
        let mut bad = restored.document().clone();
        bad.events.swap(0, 1);
        assert!(History::restore(bad).is_err());
        let mut bad = restored.document().clone();
        bad.events[1].sha256 = "0".repeat(64);
        assert!(History::restore(bad).is_err());
    }
    #[test]
    fn dangling_changes_and_unfinished_jobs_never_resume() {
        let session = Investigation::workshop();
        let mut history = History::new(&session, "Synthetic baseline").unwrap();
        assert!(
            history
                .append(Payload::Commit(vec![Change::NoteRevision {
                    record: RecordId(999),
                    previous: String::new(),
                    text: "x".into()
                }]))
                .is_err()
        );
        assert!(history.events().is_empty());
        history.start_job("pending", "fixture").unwrap();
        assert!(history.start_job("pending", "fixture").is_err());
        let restored = History::restore(history.document().clone()).unwrap();
        assert_eq!(restored.unfinished_jobs().len(), 1);
        assert!(equal(restored.session(), &session));
        assert!(restored.replay(99).is_err());
    }

    #[test]
    fn corrections_and_scoped_resolutions_replay_without_erasing_findings() {
        let mut session = Investigation::workshop();
        let original = serde_json::to_value(&session.records[3]).unwrap();
        let mut history = History::new(&session, "Synthetic baseline").unwrap();
        let correction = session
            .append_review(
                ReviewKind::Correction,
                RecordId(4),
                "The conversion establishes a value, not an intended field type.".into(),
                "No format declaration is available.".into(),
                vec![crate::domain::ObservationId::new(3).unwrap()],
            )
            .unwrap();
        history.capture(&session).unwrap();
        assert!(
            session
                .append_review(
                    ReviewKind::Resolution,
                    RecordId(5),
                    "Resolved".into(),
                    "Scoped".into(),
                    vec![]
                )
                .is_err()
        );
        session
            .append_review(
                ReviewKind::Resolution,
                RecordId(1),
                "The selected little-endian interpretation is reproducible.".into(),
                "The bytes' intended role remains unknown.".into(),
                vec![crate::domain::ObservationId::new(3).unwrap()],
            )
            .unwrap();
        session.objective = "Compare both byte orders before assigning meaning".into();
        history.capture(&session).unwrap();
        let replay = history.replay(history.events().len()).unwrap();
        assert!(equal(&replay, &session));
        assert_eq!(serde_json::to_value(&replay.records[3]).unwrap(), original);
        assert!(matches!(
            replay.records[correction].data,
            RecordData::Review {
                kind: ReviewKind::Correction,
                ..
            }
        ));
        assert!(matches!(replay.records[4].data, RecordData::Unknown { .. }));
        let mut malformed = history.document().clone();
        malformed.version = 99;
        assert!(History::restore(malformed).is_err());
    }
}
