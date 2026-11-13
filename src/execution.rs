//! Rust owns scope, sequencing and budgets. Model output is only a proposal.
use crate::{
    domain::*,
    omp::{Operation, Proposal},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProbeKey {
    pub target_sha256: String,
    pub operation: Operation,
    pub offset: u64,
    pub length: u64,
    /// Frozen request configuration, not a claim about a newly observed tool binary.
    pub profile_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scope {
    pub target_sha256: String,
    pub operation: Operation,
    pub offset: u64,
    pub length: u64,
    pub target_length: u64,
    pub profile_sha256: String,
}
impl Scope {
    pub fn validate(&self) -> Result<(), String> {
        crate::coordinates::ImageId::from_sha256(&self.target_sha256)?;
        crate::coordinates::ImageId::from_sha256(&self.profile_sha256)?;
        if self.operation == Operation::Conclude
            || self.length == 0
            || self
                .offset
                .checked_add(self.length)
                .is_none_or(|e| e > self.target_length)
        {
            return Err("Invalid execution scope".into());
        }
        if self.operation == Operation::ReadFixture
            && (self.target_sha256 != crate::fwsc::digest(&[0x34, 0x12]) || self.target_length != 2)
        {
            return Err("Fixture scope must name the compiled synthetic teaching bytes".into());
        }
        Ok(())
    }
    pub fn key(&self, proposal: &Proposal) -> Result<ProbeKey, String> {
        self.validate()?;
        if proposal.operation != self.operation
            || proposal.offset < self.offset
            || proposal.length == 0
            || proposal.length > 4096
            || proposal
                .offset
                .checked_add(proposal.length)
                .is_none_or(|e| e > self.offset + self.length)
            || (proposal.operation == Operation::Disassemble
                && (!proposal.offset.is_multiple_of(2) || !proposal.length.is_multiple_of(2)))
        {
            return Err("Proposal exceeds this run's operation or scope".into());
        }
        Ok(ProbeKey {
            target_sha256: self.target_sha256.clone(),
            operation: proposal.operation,
            offset: proposal.offset,
            length: proposal.length,
            profile_sha256: self.profile_sha256.clone(),
        })
    }
    pub fn contains(&self, key: &ProbeKey) -> bool {
        self.validate().is_ok()
            && key.target_sha256 == self.target_sha256
            && key.profile_sha256 == self.profile_sha256
            && key.operation == self.operation
            && key.length > 0
            && key.length <= 4096
            && key.offset >= self.offset
            && key
                .offset
                .checked_add(key.length)
                .is_some_and(|e| e <= self.offset + self.length)
            && (key.operation != Operation::Disassemble
                || (key.offset.is_multiple_of(2) && key.length.is_multiple_of(2)))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Running,
    Paused,
    Stopped,
    Completed,
    Failed,
}
#[derive(Clone, Debug)]
pub enum Phase {
    NeedModel {
        observation: Option<ObservationId>,
    },
    AwaitingModel {
        id: String,
        observation: Option<ObservationId>,
    },
    ReadyProbe {
        key: ProbeKey,
        proposal: RecordId,
    },
    AwaitingProbe {
        id: String,
        key: ProbeKey,
        step: u32,
        record: RecordId,
    },
    Done,
}
#[derive(Clone, Debug)]
pub struct Controller {
    pub id: String,
    pub generation: u64,
    pub question: QuestionId,
    pub scope: Scope,
    pub budget: u32,
    pub used: u32,
    pub model_requests: u32,
    pub mode: Mode,
    pub phase: Phase,
    pub status: String,
}
impl Controller {
    pub fn new(
        id: String,
        generation: u64,
        question: QuestionId,
        scope: Scope,
        budget: u32,
    ) -> Result<Self, String> {
        scope.validate()?;
        if id.is_empty() || id.len() > 200 || !(1..=32).contains(&budget) {
            return Err("A run requires an identity and a 1-32 step budget".into());
        }
        Ok(Self {
            id,
            generation,
            question,
            scope,
            budget,
            used: 0,
            model_requests: 0,
            mode: Mode::Running,
            phase: Phase::NeedModel { observation: None },
            status: "Ready to ask OMP".into(),
        })
    }
    pub fn begin_model(
        &mut self,
        id: String,
        generation: u64,
    ) -> Result<Option<ObservationId>, String> {
        if generation != self.generation
            || self.mode != Mode::Running
            || self.model_requests > self.budget
        {
            return Err("Model dispatch is stopped, stale or over budget".into());
        }
        let Phase::NeedModel { observation } = self.phase else {
            return Err("Another step already owns the controller".into());
        };
        self.model_requests += 1;
        self.phase = Phase::AwaitingModel { id, observation };
        self.status = "OMP is reading the supplied evidence".into();
        Ok(observation)
    }
    pub fn accept_model(
        &mut self,
        id: &str,
        generation: u64,
        record: RecordId,
        proposal: &Proposal,
    ) -> Result<(), String> {
        let Phase::AwaitingModel { id: expected, .. } = &self.phase else {
            return Err("No model response is expected".into());
        };
        if id != expected || generation != self.generation {
            return Err("Stale model response".into());
        }
        if self.mode == Mode::Stopped {
            self.phase = Phase::Done;
            return Ok(());
        }
        if proposal.operation == Operation::Conclude {
            self.mode = Mode::Completed;
            self.phase = Phase::Done;
            self.status = "OMP proposed a scoped conclusion; review its evidence and limits".into();
            return Ok(());
        }
        if self.used >= self.budget {
            self.mode = Mode::Completed;
            self.phase = Phase::Done;
            self.status = "Step budget exhausted; no further probe was launched".into();
            return Ok(());
        }
        let key = self.scope.key(proposal)?;
        self.phase = Phase::ReadyProbe {
            key,
            proposal: record,
        };
        self.status = "Checked proposal is ready for execution".into();
        Ok(())
    }
    pub fn begin_probe(
        &mut self,
        id: String,
        record: RecordId,
        generation: u64,
    ) -> Result<(u32, ProbeKey, RecordId), String> {
        if generation != self.generation || self.mode != Mode::Running || self.used >= self.budget {
            return Err("Probe dispatch is paused, stopped, stale or over budget".into());
        }
        let Phase::ReadyProbe { key, proposal } = &self.phase else {
            return Err("No checked probe is ready".into());
        };
        let key = key.clone();
        let proposal = *proposal;
        self.used += 1;
        self.phase = Phase::AwaitingProbe {
            id,
            key: key.clone(),
            step: self.used,
            record,
        };
        self.status = "Executing one bounded probe".into();
        Ok((self.used, key, proposal))
    }
    pub fn complete_probe(
        &mut self,
        id: &str,
        generation: u64,
        observation: Option<ObservationId>,
        complete: bool,
    ) -> Result<(), String> {
        let Phase::AwaitingProbe { id: expected, .. } = &self.phase else {
            return Err("No probe result is expected".into());
        };
        if generation != self.generation || id != expected {
            return Err("Stale probe completion".into());
        }
        if self.mode == Mode::Stopped {
            self.phase = Phase::Done;
            self.status = "Stopped; late output retained without follow-up".into();
            return Ok(());
        }
        if let Some(observation) = observation {
            self.phase = Phase::NeedModel {
                observation: Some(observation),
            };
            if !complete {
                self.mode = Mode::Paused;
                self.status = "Partial or failed probe retained. Review it before resuming".into();
            } else {
                self.status = "Observation recorded; explanation is next".into();
            }
        } else {
            self.mode = Mode::Failed;
            self.phase = Phase::Done;
            self.status = "Probe ended without accepted evidence".into();
        }
        Ok(())
    }
    pub fn pause(&mut self) {
        if self.mode == Mode::Running {
            self.mode = Mode::Paused;
            self.status = "Paused: current work may finish; no next step will launch".into();
        }
    }
    pub fn resume(&mut self) -> Result<(), String> {
        if self.mode != Mode::Paused {
            return Err("Only a paused run can resume".into());
        }
        self.mode = Mode::Running;
        self.status = "Resuming the checked run".into();
        Ok(())
    }
    pub fn stop(&mut self) {
        if matches!(self.mode, Mode::Running | Mode::Paused) {
            self.mode = Mode::Stopped;
            self.status = "Stopping current work; follow-ups are disabled".into();
            if !matches!(
                self.phase,
                Phase::AwaitingModel { .. } | Phase::AwaitingProbe { .. }
            ) {
                self.phase = Phase::Done;
            }
        }
    }
    pub fn fail(&mut self, reason: String) {
        if self.mode != Mode::Stopped {
            self.mode = Mode::Failed;
        }
        self.phase = Phase::Done;
        self.status = reason;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub cycle: String,
    pub question: QuestionId,
    pub detail: Detail,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Detail {
    Started {
        scope: Scope,
        budget: u32,
        model: String,
    },
    Paused,
    Resumed,
    Stopped,
    ProbeAccepted {
        step: u32,
        key: ProbeKey,
        proposal: RecordId,
        probe: RecordId,
    },
    ProbeRecorded {
        step: u32,
        observation: Option<ObservationId>,
        reused: bool,
        complete: bool,
        outcome: String,
    },
    ExplanationPublished {
        record: RecordId,
        request: String,
    },
    Finished {
        mode: Mode,
        used: u32,
        reason: String,
    },
}
#[derive(Clone, Debug)]
pub struct Audit {
    pub question: QuestionId,
    pub finished: bool,
    pub scope: Scope,
    pub budget: u32,
    pub used: u32,
    pub mode: Mode,
    pub pending: Option<(u32, ProbeKey, RecordId)>,
    pub completed: Vec<(ProbeKey, ObservationId)>,
}
impl Event {
    pub fn apply(&self, session: &Investigation, prior: Option<&Audit>) -> Result<Audit, String> {
        self.question.resolve(session)?;
        if self.cycle.is_empty() || self.cycle.len() > 200 {
            return Err("Invalid cycle identity".into());
        }
        if let Detail::Started {
            scope,
            budget,
            model,
        } = &self.detail
        {
            if prior.is_some()
                || !(1..=32).contains(budget)
                || model.is_empty()
                || model.len() > 300
            {
                return Err("Invalid or repeated cycle start".into());
            }
            scope.validate()?;
            let (identity, length, operation) =
                if session.origin == Origin::WorkshopFixture && session.artifact.is_none() {
                    (
                        crate::fwsc::digest(&[0x34, 0x12]),
                        2,
                        Operation::ReadFixture,
                    )
                } else {
                    let app = session
                        .artifact
                        .as_ref()
                        .and_then(|a| a.firmware.as_ref())
                        .and_then(|f| f.application.as_ref())
                        .ok_or("Cycle has no supported target")?;
                    (
                        app.sha256.clone(),
                        u64::from(app.entry().length),
                        Operation::Disassemble,
                    )
                };
            if scope.target_sha256 != identity
                || scope.target_length != length
                || scope.operation != operation
            {
                return Err("Cycle target belongs to another investigation".into());
            }
            return Ok(Audit {
                question: self.question,
                finished: false,
                scope: scope.clone(),
                budget: *budget,
                used: 0,
                mode: Mode::Running,
                pending: None,
                completed: vec![],
            });
        }
        let mut audit = prior.cloned().ok_or("Cycle event has no start")?;
        if audit.question != self.question || audit.finished {
            return Err("Cycle ownership changed or events continued after finish".into());
        }
        match &self.detail {
            Detail::Started { .. } => unreachable!(),
            Detail::Paused => {
                if audit.mode != Mode::Running {
                    return Err("Pause outside a running cycle".into());
                }
                audit.mode = Mode::Paused;
            }
            Detail::Resumed => {
                if audit.mode != Mode::Paused {
                    return Err("Resume outside a paused cycle".into());
                }
                audit.mode = Mode::Running;
            }
            Detail::Stopped => {
                if !matches!(audit.mode, Mode::Running | Mode::Paused) {
                    return Err("Stop outside an active cycle".into());
                }
                audit.mode = Mode::Stopped;
            }
            Detail::ProbeAccepted {
                step,
                key,
                proposal,
                probe,
            } => {
                if audit.mode != Mode::Running
                    || audit.pending.is_some()
                    || *step != audit.used + 1
                    || *step > audit.budget
                    || !audit.scope.contains(key)
                {
                    return Err("Probe violates budget, sequential execution or scope".into());
                }
                let receipt = session
                    .ai_runs
                    .iter()
                    .find(|r| r.record == *proposal)
                    .ok_or("Probe has no checked model proposal")?;
                let p = receipt
                    .proposal
                    .as_ref()
                    .ok_or("Missing checked proposal")?;
                if receipt.context["cycle_id"].as_str() != Some(&self.cycle)
                    || receipt.context["selected_record"].as_u64() != Some(self.question.value())
                    || receipt.context["allowed_scope"]["offset"].as_u64()
                        != Some(audit.scope.offset)
                    || receipt.context["allowed_scope"]["length"].as_u64()
                        != Some(audit.scope.length)
                {
                    return Err("Proposal belongs to another cycle, question or scope".into());
                }
                if audit.scope.key(p)? != *key
                    || !session.record(*probe).is_some_and(|r| {
                        matches!(r.data, RecordData::Probe { .. })
                            && r.question_id == Some(self.question)
                    })
                {
                    return Err("Probe event differs from its proposal or graph action".into());
                }
                audit.used = *step;
                audit.pending = Some((*step, key.clone(), *probe));
            }
            Detail::ProbeRecorded {
                step,
                observation,
                reused,
                complete,
                outcome,
            } => {
                let (pending, key, probe) = audit
                    .pending
                    .take()
                    .ok_or("Probe result has no accepted request")?;
                if pending != *step
                    || outcome.is_empty()
                    || outcome.len() > 4000
                    || (*reused && !*complete)
                {
                    return Err("Invalid probe result event".into());
                }
                if let Some(id) = observation {
                    validate_observation(session, &key, *id, *complete)?;
                    let relation = if *reused {
                        Relation::Reuses
                    } else {
                        Relation::Produces
                    };
                    if !session
                        .edges
                        .iter()
                        .any(|e| e.from == probe && e.to == id.record() && e.relation == relation)
                    {
                        return Err("Observation has no matching probe provenance edge".into());
                    }
                    if *complete {
                        audit.completed.push((key, *id));
                    }
                } else if *complete {
                    return Err("Completed probe has no evidence".into());
                }
            }
            Detail::ExplanationPublished { record, request } => {
                if audit.mode == Mode::Stopped
                    || !session.ai_runs.iter().any(|r| {
                        r.id == *request
                            && r.analysis_record == Some(*record)
                            && r.context["cycle_id"].as_str() == Some(&self.cycle)
                            && r.context["selected_record"].as_u64() == Some(self.question.value())
                    })
                {
                    return Err("Explanation has no matching model receipt".into());
                }
            }
            Detail::Finished { mode, used, reason } => {
                if *used != audit.used
                    || reason.is_empty()
                    || reason.len() > 4000
                    || audit.pending.is_some()
                    || !matches!(mode, Mode::Completed | Mode::Stopped | Mode::Failed)
                    || ((audit.mode == Mode::Stopped) != (*mode == Mode::Stopped))
                {
                    return Err("Invalid final cycle accounting".into());
                }
                audit.mode = *mode;
                audit.finished = true;
            }
        }
        Ok(audit)
    }
    pub fn summary(&self) -> String {
        format!("AI run {} / {:?}", self.cycle, self.detail)
            .chars()
            .take(180)
            .collect()
    }
}

pub fn validate_observation(
    session: &Investigation,
    key: &ProbeKey,
    id: ObservationId,
    complete: bool,
) -> Result<(), String> {
    let record = id.resolve(session)?;
    match (&record.data, key.operation) {
        (RecordData::Disassembly(run), Operation::Disassemble) => {
            let r = session
                .disassemblies
                .iter()
                .find(|r| r.id == *run)
                .ok_or("Missing run")?;
            if r.scope.offset.0 != key.offset
                || r.scope.length != key.length
                || r.map.image_id.as_str() != key.target_sha256
                || (complete
                    && (r.capture.outcome != crate::tool_runner::ToolOutcome::Completed
                        || r.coverage != crate::pi32::Coverage::Complete))
            {
                return Err("Cached result or completion differs from the checked probe".into());
            }
        }
        (RecordData::Observation(observation), Operation::ReadFixture) => {
            let end = key
                .offset
                .checked_add(key.length)
                .ok_or("Fixture range overflow")?;
            let start = usize::try_from(key.offset).map_err(|_| "Fixture offset too large")?;
            let end = usize::try_from(end).map_err(|_| "Fixture end too large")?;
            if session.origin != Origin::WorkshopFixture
                || session.artifact.is_some()
                || key.target_sha256 != crate::fwsc::digest(&[0x34, 0x12])
                || observation.offset.0 != key.offset
                || [0x34, 0x12].get(start..end) != Some(observation.bytes.as_slice())
            {
                return Err("Fixture observation differs from the synthetic source".into());
            }
        }
        _ => return Err("Probe result has the wrong evidence kind".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope() -> Scope {
        Scope {
            target_sha256: crate::fwsc::digest(&[0x34, 0x12]),
            operation: Operation::ReadFixture,
            offset: 0,
            length: 2,
            target_length: 2,
            profile_sha256: crate::fwsc::digest(b"fixture-v1"),
        }
    }
    fn proposal() -> Proposal {
        Proposal {
            operation: Operation::ReadFixture,
            analysis: None,
            question: "What are the bytes?".into(),
            why: "Observe".into(),
            method: "Read fixture".into(),
            enables: "Compare".into(),
            unknowns: "Meaning".into(),
            evidence: vec![ObservationId::new(3).unwrap()],
            offset: 0,
            length: 2,
        }
    }
    #[test]
    fn pause_stop_stale_completion_and_budget_are_independent() {
        let mut c =
            Controller::new("cycle".into(), 7, QuestionId::new(1).unwrap(), scope(), 1).unwrap();
        c.begin_model("m1".into(), 7).unwrap();
        c.pause();
        c.accept_model("m1", 7, RecordId(10), &proposal()).unwrap();
        assert!(c.begin_probe("p1".into(), RecordId(11), 7).is_err());
        c.resume().unwrap();
        c.begin_probe("p1".into(), RecordId(11), 7).unwrap();
        assert!(
            c.complete_probe("p1", 8, Some(ObservationId::new(3).unwrap()), true)
                .is_err()
        );
        c.complete_probe("p1", 7, Some(ObservationId::new(3).unwrap()), true)
            .unwrap();
        c.begin_model("m2".into(), 7).unwrap();
        c.accept_model("m2", 7, RecordId(12), &proposal()).unwrap();
        assert_eq!(c.used, 1);
        assert_eq!(c.mode, Mode::Completed);
        assert!(c.begin_probe("p2".into(), RecordId(13), 7).is_err());
        let mut c =
            Controller::new("stop".into(), 7, QuestionId::new(1).unwrap(), scope(), 12).unwrap();
        c.begin_model("m".into(), 7).unwrap();
        c.stop();
        c.accept_model("m", 7, RecordId(10), &proposal()).unwrap();
        assert!(matches!(c.phase, Phase::Done));
        assert_eq!(c.mode, Mode::Stopped);
        assert!(c.begin_model("late".into(), 7).is_err());
    }
    #[test]
    fn partial_failure_pauses_and_overflow_never_becomes_a_request() {
        let mut c = Controller::new(
            "partial".into(),
            1,
            QuestionId::new(1).unwrap(),
            scope(),
            12,
        )
        .unwrap();
        c.begin_model("m".into(), 1).unwrap();
        c.accept_model("m", 1, RecordId(10), &proposal()).unwrap();
        c.begin_probe("p".into(), RecordId(11), 1).unwrap();
        c.complete_probe("p", 1, Some(ObservationId::new(3).unwrap()), false)
            .unwrap();
        assert_eq!(c.mode, Mode::Paused);
        assert!(c.begin_model("follow".into(), 1).is_err());
        let mut p = proposal();
        p.offset = u64::MAX;
        assert!(scope().key(&p).is_err());
        p = proposal();
        p.operation = Operation::Disassemble;
        assert!(scope().key(&p).is_err());
        assert!(Controller::new("x".into(), 0, QuestionId::new(1).unwrap(), scope(), 0).is_err());
    }
}
