//! Sequential live runtime for the checked controller. No saved state starts this runtime.
use crate::{
    coordinates::ImageOffset,
    decoded_image::DecodedImage,
    domain::*,
    execution::{self, Controller, Detail, Event, Mode, Phase, ProbeKey, Scope},
    history::{History, ReviewKind},
    omp::{self, Operation},
    pi32::{self, DisassemblyReport, RunId},
    tool_runner::{ToolConfig, ToolOutcome},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub enum Target {
    V15(Arc<DecodedImage>),
    Teaching,
}
impl Target {
    pub fn from_session(
        session: &Investigation,
        image: Option<Arc<DecodedImage>>,
    ) -> Result<Self, String> {
        if session.origin == Origin::WorkshopFixture && session.artifact.is_none() {
            return Ok(Self::Teaching);
        }
        let image = image.ok_or("Verify the original source before starting live V15 execution")?;
        let artifact = session.artifact.as_ref().ok_or("No artifact")?;
        if image.map().artifact_id.as_str() != artifact.sha256
            || artifact
                .firmware
                .as_ref()
                .and_then(|f| f.application.as_ref())
                .is_none_or(|a| a.sha256 != image.map().image_id.as_str())
        {
            return Err("Fresh image belongs to a different investigation".into());
        }
        Ok(Self::V15(image))
    }
    pub fn identity(&self) -> String {
        match self {
            Self::V15(i) => i.map().image_id.as_str().into(),
            Self::Teaching => crate::fwsc::digest(&[0x34, 0x12]),
        }
    }
    pub fn length(&self) -> u64 {
        match self {
            Self::V15(i) => i.map().image_length,
            Self::Teaching => 2,
        }
    }
    pub fn operation(&self) -> Operation {
        match self {
            Self::V15(_) => Operation::Disassemble,
            Self::Teaching => Operation::ReadFixture,
        }
    }
}
#[derive(Clone)]
pub struct Settings {
    pub question: QuestionId,
    pub model: String,
    pub budget: u32,
    pub offset: u64,
    pub length: u64,
    pub objective: String,
}
enum Output {
    Disassembly(Box<DisassemblyReport>),
    Fixture(Observation),
}
struct ProbeJob {
    id: String,
    generation: u64,
    cancel: Arc<AtomicBool>,
    receiver: mpsc::Receiver<Result<Output, String>>,
}
impl Drop for ProbeJob {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Default)]
pub struct Engine {
    pub controller: Option<Controller>,
    pub draft: String,
    model: String,
    target: Option<Target>,
    config: ToolConfig,
    model_job: Option<omp::Job>,
    probe_job: Option<ProbeJob>,
    cache: HashMap<ProbeKey, ObservationId>,
    finished_recorded: bool,
}
fn unique(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}
impl Engine {
    pub fn busy(&self) -> bool {
        self.model_job.is_some()
            || self.probe_job.is_some()
            || self
                .controller
                .as_ref()
                .is_some_and(|c| matches!(c.mode, Mode::Running | Mode::Paused))
    }
    pub fn has_worker(&self) -> bool {
        self.model_job.is_some() || self.probe_job.is_some()
    }
    pub fn start(
        &mut self,
        session: &mut Investigation,
        history: &mut History,
        target: Target,
        config: ToolConfig,
        settings: Settings,
        generation: u64,
    ) -> Result<(), String> {
        if self.busy() {
            return Err("Stop or finish the current cycle first".into());
        }
        settings.question.resolve(session)?;
        if settings.model.is_empty()
            || settings.model.len() > 300
            || settings.objective.trim().is_empty()
            || settings.objective.len() > 4000
        {
            return Err("Choose a configured model and a bounded objective".into());
        }
        let scope = Scope {
            target_sha256: target.identity(),
            operation: target.operation(),
            offset: settings.offset,
            length: settings.length,
            target_length: target.length(),
            profile_sha256: crate::fwsc::digest(
                &serde_json::to_vec(&(target.operation(), &config, "probe-profile-v1")).unwrap(),
            ),
        };
        let controller = Controller::new(
            unique("cycle"),
            generation,
            settings.question,
            scope,
            settings.budget,
        )?;
        ensure_capacity(session, history)?;
        if session.objective != settings.objective {
            session.append_review(
                ReviewKind::Steering,
                settings.question.record(),
                format!("Set the investigation objective: {}", settings.objective),
                "The objective directs probes; it is not evidence that the answer is known".into(),
                vec![],
            )?;
            session.objective = settings.objective;
        }
        history.capture(session)?;
        history.execution_event(Event {
            cycle: controller.id.clone(),
            question: controller.question,
            detail: Detail::Started {
                scope: controller.scope.clone(),
                budget: controller.budget,
                model: settings.model.clone(),
            },
        })?;
        self.cache = history
            .completed_probes()
            .filter(|(key, id)| execution::validate_observation(session, key, *id, true).is_ok())
            .cloned()
            .collect();
        self.model = settings.model;
        self.target = Some(target);
        self.config = config;
        self.controller = Some(controller);
        self.draft.clear();
        self.finished_recorded = false;
        Ok(())
    }
    fn event(&self, history: &mut History, detail: Detail) -> Result<(), String> {
        let c = self.controller.as_ref().ok_or("No execution controller")?;
        history.execution_event(Event {
            cycle: c.id.clone(),
            question: c.question,
            detail,
        })
    }
    pub fn pause(&mut self, history: &mut History) -> Result<(), String> {
        if self
            .controller
            .as_ref()
            .is_some_and(|c| c.mode == Mode::Running)
        {
            self.event(history, Detail::Paused)?;
            self.controller.as_mut().unwrap().pause();
        }
        Ok(())
    }
    pub fn resume(&mut self, history: &mut History) -> Result<(), String> {
        if self
            .controller
            .as_ref()
            .is_none_or(|c| c.mode != Mode::Paused)
        {
            return Err("No paused cycle".into());
        }
        self.event(history, Detail::Resumed)?;
        self.controller.as_mut().unwrap().resume()
    }
    pub fn stop(&mut self, history: &mut History) -> Result<(), String> {
        if self
            .controller
            .as_ref()
            .is_some_and(|c| matches!(c.mode, Mode::Running | Mode::Paused))
        {
            self.event(history, Detail::Stopped)?;
            self.controller.as_mut().unwrap().stop();
        }
        if let Some(job) = &self.model_job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        if let Some(job) = &self.probe_job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
    pub fn invalidate(&mut self) {
        self.model_job = None;
        self.probe_job = None;
        self.controller = None;
        self.target = None;
        self.draft.clear();
    }
    pub fn snapshot(&self, session: &Investigation) -> Result<Value, String> {
        let c = self.controller.as_ref().ok_or("No run")?;
        let required = match c.phase {
            Phase::NeedModel { observation } | Phase::AwaitingModel { observation, .. } => {
                observation
            }
            _ => None,
        };
        let mut context = omp::context(session, session.records.len(), c.question.record())?;
        context["cycle_id"] = json!(c.id);
        context["allowed_scope"] = json!({"offset":c.scope.offset,"length":c.scope.length});
        context["allowed_operations"] = if c.used >= c.budget {
            json!(["conclude"])
        } else {
            json!([c.scope.operation.label(), "conclude"])
        };
        context["remaining_probe_budget"] = json!(c.budget - c.used);
        context["required_observation"] = json!(required.map(|id| id.value()));
        context["question_plan"] = json!(c.question.resolve(session)?.question_plan);
        let mut cached: Vec<_> = self
            .cache
            .iter()
            .filter(|(key, _)| c.scope.contains(key))
            .collect();
        cached.sort_by_key(|(key, id)| (key.offset, key.length, id.value()));
        context["completed_probes"]=json!(cached.into_iter().take(32).map(|(key,id)|json!({"operation":key.operation,"offset":key.offset,"length":key.length,"observation":id})).collect::<Vec<_>>());
        if let Some(id) = required
            && !context["record_ids"]
                .as_array()
                .unwrap()
                .contains(&json!(id.value()))
        {
            let r = id.resolve(session)?;
            if context["record_ids"].as_array().unwrap().len() >= 12 {
                let omitted = context["record_ids"].as_array_mut().unwrap().pop().unwrap();
                context["evidence"].as_array_mut().unwrap().pop();
                context["observation_replaced_by_required_result"] = omitted;
            }
            context["record_ids"]
                .as_array_mut()
                .unwrap()
                .push(json!(id.value()));
            context["evidence"].as_array_mut().unwrap().push(json!({"id":id,"title":crate::memory::clip(&r.title,180),"observation":crate::memory::clip(&r.insight.observation,1200),"uncertainty":crate::memory::clip(&r.insight.uncertainty,900)}));
        }
        if serde_json::to_vec(&context).unwrap().len() > 64000 {
            return Err("Execution context exceeds 64 KiB".into());
        }
        Ok(context)
    }
    /// Process terminal messages first; dispatch at most one subsequent worker per tick.
    pub fn tick(
        &mut self,
        session: &mut Investigation,
        history: &mut History,
        generation: u64,
    ) -> Result<bool, String> {
        let before = history.events().len();
        let Some(c) = &self.controller else {
            return Ok(false);
        };
        if c.generation != generation {
            self.invalidate();
            return Ok(false);
        }
        let result = self.advance(session, history, generation);
        if let Err(error) = &result {
            self.abort(history, error)?;
        }
        result?;
        Ok(history.events().len() != before)
    }
    fn abort(&mut self, history: &mut History, error: &str) -> Result<(), String> {
        // A failed publication ends the run; detached workers cannot publish through a new cycle.
        let reason: String = error.chars().take(1000).collect();
        self.model_job = None;
        self.probe_job = None;
        let phase = self.controller.as_ref().unwrap().phase.clone();
        if let Phase::AwaitingModel { id, .. } | Phase::AwaitingProbe { id, .. } = &phase
            && history.unfinished_jobs().contains_key(id)
        {
            history.finish_job(id, &reason)?;
        }
        if let Phase::AwaitingProbe { step, .. } = phase
            && history
                .pending_probe(&self.controller.as_ref().unwrap().id)
                .is_some()
        {
            self.event(
                history,
                Detail::ProbeRecorded {
                    step,
                    observation: None,
                    reused: false,
                    complete: false,
                    outcome: format!("Publication failed: {reason}"),
                },
            )?;
        }
        self.controller.as_mut().unwrap().fail(reason);
        Ok(())
    }
    fn advance(
        &mut self,
        session: &mut Investigation,
        history: &mut History,
        generation: u64,
    ) -> Result<(), String> {
        if let Some(job) = &mut self.model_job
            && let Some(run) = omp::poll_proposal(job, &mut self.draft)
        {
            let job = self.model_job.take().unwrap();
            let id = job.id.clone();
            let outcome = run.outcome.clone();
            let proposal = run.proposal.clone();
            let index = session.append_ai_run(run)?;
            history.capture(session)?;
            history.finish_job(&id, &outcome)?;
            if let Some(record) = session.ai_runs.last().unwrap().analysis_record {
                self.event(
                    history,
                    Detail::ExplanationPublished {
                        record,
                        request: id.clone(),
                    },
                )?;
            }
            if let Some(proposal) = proposal {
                self.controller.as_mut().unwrap().accept_model(
                    &id,
                    generation,
                    session.records[index].id,
                    &proposal,
                )?;
            } else {
                self.controller
                    .as_mut()
                    .unwrap()
                    .fail(format!("OMP {outcome}; no next probe was dispatched"));
            }
        }
        if let Some(job) = &self.probe_job {
            let received = match job.receiver.try_recv() {
                Ok(value) => Some(value),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Probe worker disconnected".into()))
                }
            };
            if let Some(result) = received {
                let job = self.probe_job.take().unwrap();
                if job.generation != generation {
                    return Err("Stale probe generation".into());
                }
                let Phase::AwaitingProbe {
                    key, step, record, ..
                } = self.controller.as_ref().unwrap().phase.clone()
                else {
                    return Err("Probe result has no owning phase".into());
                };
                let question = self.controller.as_ref().unwrap().question;
                let (observation, complete, outcome) =
                    append_output(session, question, record, &key, result)?;
                history.capture(session)?;
                history.finish_job(&job.id, &outcome)?;
                self.record_probe(
                    history,
                    &job.id,
                    generation,
                    step,
                    key,
                    observation,
                    false,
                    complete,
                    outcome,
                )?;
            }
        }
        let c = self.controller.as_ref().unwrap();
        if !self.has_worker() && matches!(c.phase, Phase::Done) && !self.finished_recorded {
            self.event(
                history,
                Detail::Finished {
                    mode: c.mode,
                    used: c.used,
                    reason: c.status.clone(),
                },
            )?;
            self.finished_recorded = true;
            return Ok(());
        }
        if c.mode != Mode::Running || self.has_worker() {
            return Ok(());
        }
        match c.phase.clone() {
            Phase::NeedModel { .. } => {
                ensure_capacity(session, history)?;
                history.capture(session)?;
                let context = self.snapshot(session)?;
                self.draft.clear();
                let job = omp::start(context, self.model.clone(), true, generation);
                self.controller
                    .as_mut()
                    .unwrap()
                    .begin_model(job.id.clone(), generation)?;
                history.start_job(&job.id, "OMP bounded cycle")?;
                self.model_job = Some(job);
            }
            Phase::ReadyProbe { key, proposal } => {
                ensure_capacity(session, history)?;
                let question = c.question;
                let probe = append_probe(
                    session,
                    question,
                    proposal,
                    &key,
                    self.cache.contains_key(&key),
                )?;
                history.capture(session)?;
                let id = unique("probe");
                let (step, key, proposal) =
                    self.controller
                        .as_mut()
                        .unwrap()
                        .begin_probe(id.clone(), probe, generation)?;
                self.event(
                    history,
                    Detail::ProbeAccepted {
                        step,
                        key: key.clone(),
                        proposal,
                        probe,
                    },
                )?;
                if let Some(observation) = self.cache.get(&key).copied() {
                    execution::validate_observation(session, &key, observation, true)?;
                    session.edges.push(Edge {
                        from: probe,
                        to: observation.record(),
                        relation: Relation::Reuses,
                    });
                    history.capture(session)?;
                    self.record_probe(
                        history,
                        &id,
                        generation,
                        step,
                        key,
                        Some(observation),
                        true,
                        true,
                        "Reused an identical completed request; original tool receipt retained"
                            .into(),
                    )?;
                } else {
                    history.start_job(&id, key.operation.label())?;
                    let cancel = Arc::new(AtomicBool::new(false));
                    let flag = cancel.clone();
                    let target = self.target.clone().ok_or("No fresh target")?;
                    let config = self.config.clone();
                    let run = RunId(
                        session
                            .disassemblies
                            .iter()
                            .map(|r| r.id.0)
                            .max()
                            .unwrap_or(0)
                            .checked_add(1)
                            .ok_or("Run ID overflow")?,
                    );
                    let (send, receiver) = mpsc::channel();
                    std::thread::spawn(move || {
                        let result = if flag.load(Ordering::Relaxed) {
                            Err("Cancelled before probe launch".into())
                        } else {
                            match target {
                                Target::Teaching => {
                                    let bytes = [0x34, 0x12];
                                    bytes.get(key.offset as usize..(key.offset+key.length) as usize).map(|b|Output::Fixture(Observation{offset:FileOffset(key.offset),bytes:b.to_vec(),description:"Fresh read of compiled synthetic workshop bytes; not firmware".into()})).ok_or("Fixture range is outside the source".into())
                                }
                                Target::V15(image) => pi32::disassemble(
                                    &image,
                                    run,
                                    ImageOffset(key.offset),
                                    key.length,
                                    &config,
                                    &flag,
                                )
                                .map(|r| Output::Disassembly(Box::new(r))),
                            }
                        };
                        let _ = send.send(result);
                    });
                    self.probe_job = Some(ProbeJob {
                        id,
                        generation,
                        cancel,
                        receiver,
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn record_probe(
        &mut self,
        history: &mut History,
        id: &str,
        generation: u64,
        step: u32,
        key: ProbeKey,
        observation: Option<ObservationId>,
        reused: bool,
        complete: bool,
        outcome: String,
    ) -> Result<(), String> {
        self.event(
            history,
            Detail::ProbeRecorded {
                step,
                observation,
                reused,
                complete,
                outcome,
            },
        )?;
        if complete && let Some(observation) = observation {
            self.cache.insert(key, observation);
        }
        let previous = self.controller.as_ref().unwrap().mode;
        self.controller
            .as_mut()
            .unwrap()
            .complete_probe(id, generation, observation, complete)?;
        if previous == Mode::Running && self.controller.as_ref().unwrap().mode == Mode::Paused {
            self.event(history, Detail::Paused)?;
        }
        Ok(())
    }
}

fn ensure_capacity(session: &Investigation, history: &History) -> Result<(), String> {
    if session.records.len() + 8 > 200
        || session.ai_runs.len() >= 16
        || session.disassemblies.len() >= 16
        || history.events().len() + 16 > 4096
    {
        return Err(
            "Case recording capacity reached; save and review before starting more work".into(),
        );
    }
    Ok(())
}
fn next_id(session: &Investigation) -> Result<RecordId, String> {
    Ok(RecordId(
        session
            .records
            .iter()
            .map(|r| r.id.0)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Record ID overflow")?,
    ))
}
fn append_probe(
    session: &mut Investigation,
    question: QuestionId,
    proposal: RecordId,
    key: &ProbeKey,
    reused: bool,
) -> Result<RecordId, String> {
    let previous = session.clone();
    let result = append_probe_inner(session, question, proposal, key, reused);
    if result.is_err() {
        *session = previous;
    }
    result
}
fn append_probe_inner(
    session: &mut Investigation,
    question: QuestionId,
    proposal: RecordId,
    key: &ProbeKey,
    reused: bool,
) -> Result<RecordId, String> {
    let p = session.record(proposal).ok_or("Missing proposal")?.clone();
    let id = next_id(session)?;
    let mut insight = p.insight.clone();
    insight.action = format!(
        "{} {} at offset 0x{:X}, {} bytes",
        if reused {
            "Reuse a completed"
        } else {
            "Execute checked"
        },
        key.operation.label(),
        key.offset,
        key.length
    );
    insight.observation =
        "The result is recorded in the following observation; this action alone is not evidence"
            .into();
    session.records.push(Record {
        id,
        title: if reused {
            "Reuse completed probe".into()
        } else {
            "Execute checked probe".into()
        },
        summary: format!(
            "{} / +0x{:X} / {} bytes",
            key.operation.label(),
            key.offset,
            key.length
        ),
        data: RecordData::Probe {
            operation: key.operation.label().into(),
        },
        insight,
        note: String::new(),
        question_id: Some(question),
        question_plan: None,
    });
    session.edges.push(Edge {
        from: proposal,
        to: id,
        relation: Relation::Investigates,
    });
    session.validate()?;
    Ok(id)
}
fn append_output(
    session: &mut Investigation,
    question: QuestionId,
    probe: RecordId,
    key: &ProbeKey,
    result: Result<Output, String>,
) -> Result<(Option<ObservationId>, bool, String), String> {
    let previous = session.clone();
    let result = append_output_inner(session, question, probe, key, result);
    if result.is_err() {
        *session = previous;
    }
    result
}
fn append_output_inner(
    session: &mut Investigation,
    question: QuestionId,
    probe: RecordId,
    key: &ProbeKey,
    result: Result<Output, String>,
) -> Result<(Option<ObservationId>, bool, String), String> {
    let id = next_id(session)?;
    let mut insight = session
        .record(probe)
        .ok_or("Missing probe record")?
        .insight
        .clone();
    let (data, title, complete, outcome) = match result {
        Ok(Output::Fixture(observation)) => {
            insight.observation = format!(
                "Synthetic file+0x{:X}: {}",
                observation.offset.0,
                observation
                    .bytes
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            insight.explanation="The bytes were read from the compiled teaching fixture. Their intended meaning is not established by reading them".into();
            insight.uncertainty =
                "Synthetic teaching data only; no FM-1 or runtime behavior was observed".into();
            (
                RecordData::Observation(observation),
                "Synthetic bytes observed".into(),
                true,
                "Completed synthetic byte read".into(),
            )
        }
        Ok(Output::Disassembly(report)) => {
            report.validate()?;
            let complete = report.capture.outcome == ToolOutcome::Completed
                && report.coverage == pi32::Coverage::Complete;
            let outcome = format!(
                "{:?} / {:?} coverage",
                report.capture.outcome, report.coverage
            );
            insight.observation = format!(
                "{} instructions, {} of {} bytes decoded; {outcome}",
                report.instructions.len(),
                report.decoded_bytes(),
                report.scope.length
            );
            insight.explanation="Accepted instruction bytes match the verified application span. Original source mappings, tool output and conservative static edges are retained".into();
            insight.uncertainty="Static decoding does not prove execution or function purpose. Gaps and unsupported transfers remain unresolved".into();
            let run = report.id;
            session.disassemblies.push(*report);
            (
                RecordData::Disassembly(run),
                "Bounded instruction result".into(),
                complete,
                outcome,
            )
        }
        Err(error) => {
            insight.observation = error.clone();
            insight.explanation = "No accepted probe evidence was produced".into();
            insight.uncertainty = "The requested question remains unanswered".into();
            (
                RecordData::Unknown {
                    next_probe: "Review the failure before retrying".into(),
                },
                "Probe did not produce evidence".into(),
                false,
                error,
            )
        }
    };
    insight.next_step =
        "Ask OMP to explain the actual observation and choose a distinguishing next probe".into();
    let evidence = matches!(
        data,
        RecordData::Observation(_) | RecordData::Disassembly(_)
    );
    session.records.push(Record {
        id,
        title,
        summary: outcome.clone(),
        data,
        insight,
        note: String::new(),
        question_id: Some(question),
        question_plan: None,
    });
    session.edges.push(Edge {
        from: probe,
        to: id,
        relation: Relation::Produces,
    });
    session.validate()?;
    let observation = if evidence {
        Some(ObservationId::new(id.0)?)
    } else {
        None
    };
    if let Some(observation) = observation {
        execution::validate_observation(session, key, observation, complete)?;
    }
    Ok((observation, complete, outcome))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (Investigation, History, Engine) {
        let mut s = Investigation::workshop();
        let mut h = History::new(&s, "Synthetic test baseline").unwrap();
        let mut e = Engine::default();
        let objective = s.objective.clone();
        e.start(
            &mut s,
            &mut h,
            Target::Teaching,
            ToolConfig::default(),
            Settings {
                question: QuestionId::new(1).unwrap(),
                model: "test/model".into(),
                budget: 2,
                offset: 0,
                length: 2,
                objective,
            },
            7,
        )
        .unwrap();
        (s, h, e)
    }
    fn proposal(evidence: ObservationId, conclude: bool, analysis: bool) -> omp::Proposal {
        omp::Proposal {
            operation: if conclude {
                Operation::Conclude
            } else {
                Operation::ReadFixture
            },
            question: "What are the two bytes?".into(),
            why: "Observe before interpreting".into(),
            method: "Read the bounded fixture".into(),
            enables: "Compare byte order".into(),
            unknowns: "Intended meaning is unproved".into(),
            evidence: vec![evidence],
            offset: 0,
            length: if conclude { 0 } else { 2 },
            analysis: analysis.then(|| omp::Analysis {
                explanation: "The observed bytes 34 12 can represent 4660 in little-endian u16"
                    .into(),
                significance: "Representation can be checked independently of semantic meaning"
                    .into(),
                evidence: vec![evidence],
                limitations: "Intended type and behavior are not proved".into(),
            }),
        }
    }
    fn inject(e: &mut Engine, h: &mut History, s: &Investigation, data: Value) {
        let context = e.snapshot(s).unwrap();
        let id = unique("test-model");
        e.controller
            .as_mut()
            .unwrap()
            .begin_model(id.clone(), 7)
            .unwrap();
        h.start_job(&id, "synthetic model transport").unwrap();
        let (send, receiver) = mpsc::channel();
        let events = [
            (
                "ready",
                json!({"sdk_version":"test","cli_version":"test","selected":"test/model","models":[],"active_tools":["read_evidence"]}),
            ),
            ("evidence_read", json!({"record_ids":context["record_ids"]})),
            ("proposal", data),
            ("completed", Value::Null),
        ];
        for (n, (kind, data)) in events.into_iter().enumerate() {
            send.send(Ok(omp::Event {
                v: 1,
                id: id.clone(),
                seq: n as u64 + 1,
                kind: kind.into(),
                data,
            }))
            .unwrap();
        }
        e.model_job = Some(omp::Job {
            receiver,
            cancel: Arc::new(AtomicBool::new(false)),
            id,
            context,
            events: vec![],
            generation: 7,
            propose: true,
        });
    }
    fn finish_probe_paused(
        e: &mut Engine,
        h: &mut History,
        s: &mut Investigation,
    ) -> ObservationId {
        e.pause(h).unwrap();
        for _ in 0..200 {
            e.tick(s, h, 7).unwrap();
            if !e.has_worker() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(!e.has_worker());
        assert_eq!(e.controller.as_ref().unwrap().mode, Mode::Paused);
        match e.controller.as_ref().unwrap().phase {
            Phase::NeedModel {
                observation: Some(id),
            } => id,
            _ => panic!("No completed observation"),
        }
    }
    #[test]
    fn real_fixture_probe_cache_explanation_and_offline_replay() {
        let (mut s, mut h, mut e) = setup();
        let p = proposal(ObservationId::new(3).unwrap(), false, false);
        inject(&mut e, &mut h, &s, json!(p));
        e.pause(&mut h).unwrap();
        e.tick(&mut s, &mut h, 7).unwrap();
        assert!(matches!(
            e.controller.as_ref().unwrap().phase,
            Phase::ReadyProbe { .. }
        ));
        assert!(!e.has_worker());
        e.resume(&mut h).unwrap();
        e.tick(&mut s, &mut h, 7).unwrap();
        let observed = finish_probe_paused(&mut e, &mut h, &mut s);
        let before = s
            .records
            .iter()
            .filter(|r| matches!(r.data, RecordData::Observation(_)))
            .count();
        e.resume(&mut h).unwrap();
        inject(&mut e, &mut h, &s, json!(proposal(observed, false, true)));
        e.tick(&mut s, &mut h, 7).unwrap(); // Same request reuses the original observation.
        assert!(!e.has_worker());
        assert_eq!(e.controller.as_ref().unwrap().used, 2);
        assert_eq!(
            before,
            s.records
                .iter()
                .filter(|r| matches!(r.data, RecordData::Observation(_)))
                .count()
        );
        assert!(s.edges.iter().any(|edge| edge.relation == Relation::Reuses));
        inject(&mut e, &mut h, &s, json!(proposal(observed, true, true)));
        e.tick(&mut s, &mut h, 7).unwrap();
        assert_eq!(e.controller.as_ref().unwrap().mode, Mode::Completed);
        assert_eq!(
            s.ai_runs
                .iter()
                .filter(|r| r.analysis_record.is_some())
                .count(),
            2
        );
        assert!(h.unfinished_jobs().is_empty());
        let replay = History::restore(h.document().clone()).unwrap();
        assert_eq!(json!(replay.session()), json!(s));
        for i in 0..=h.events().len() {
            h.replay(i).unwrap();
        }
        let mut loaded = Engine::default();
        assert!(!loaded.tick(&mut s, &mut h, 7).unwrap()); // Saved data cannot start a worker.
    }
    #[test]
    fn malformed_response_and_stop_race_never_dispatch() {
        let (mut s, mut h, mut e) = setup();
        let mut invalid = json!(proposal(ObservationId::new(3).unwrap(), false, false));
        invalid["unknowns"] = json!(["wrong type"]);
        inject(&mut e, &mut h, &s, invalid);
        e.tick(&mut s, &mut h, 7).unwrap();
        assert_eq!(e.controller.as_ref().unwrap().mode, Mode::Failed);
        assert_eq!(e.controller.as_ref().unwrap().used, 0);
        assert_eq!(s.ai_runs.last().unwrap().outcome, "invalid_proposal");
        assert!(s.ai_runs.last().unwrap().diagnostic.contains("string"));
        History::restore(h.document().clone()).unwrap();
        let (mut s, mut h, mut e) = setup();
        inject(
            &mut e,
            &mut h,
            &s,
            json!(proposal(ObservationId::new(3).unwrap(), false, false)),
        );
        e.stop(&mut h).unwrap();
        e.tick(&mut s, &mut h, 7).unwrap();
        assert_eq!(e.controller.as_ref().unwrap().mode, Mode::Stopped);
        assert_eq!(e.controller.as_ref().unwrap().used, 0);
        assert_eq!(s.ai_runs[0].outcome, "cancelled");
        assert!(s.ai_runs[0].proposal.is_none());
        assert!(!e.busy());
        History::restore(h.document().clone()).unwrap();
    }
    #[test]
    fn stale_generation_and_rejected_output_do_not_publish() {
        let (mut s, mut h, mut e) = setup();
        inject(
            &mut e,
            &mut h,
            &s,
            json!(proposal(ObservationId::new(3).unwrap(), false, false)),
        );
        let before = json!(s);
        e.tick(&mut s, &mut h, 8).unwrap();
        assert_eq!(before, json!(s));
        assert!(!e.busy());
        let key = ProbeKey {
            target_sha256: crate::fwsc::digest(&[0x34, 0x12]),
            operation: Operation::ReadFixture,
            offset: 0,
            length: 2,
            profile_sha256: crate::fwsc::digest(b"test"),
        };
        assert!(
            append_output(
                &mut s,
                QuestionId::new(1).unwrap(),
                RecordId(2),
                &key,
                Ok(Output::Fixture(Observation {
                    offset: FileOffset(0),
                    bytes: vec![1, 2],
                    description: "Wrong bytes".into()
                }))
            )
            .is_err()
        );
        assert_eq!(before, json!(s));
    }
    #[test]
    fn late_probe_after_stop_and_worker_failure_have_final_accounting() {
        for failed in [false, true] {
            let (mut s, mut h, mut e) = setup();
            inject(
                &mut e,
                &mut h,
                &s,
                json!(proposal(ObservationId::new(3).unwrap(), false, false)),
            );
            e.tick(&mut s, &mut h, 7).unwrap();
            assert!(e.probe_job.is_some());
            if failed {
                let job = e.probe_job.take().unwrap();
                let (send, receiver) = mpsc::channel();
                send.send(Err(
                    "Synthetic tool failure with no accepted evidence".into()
                ))
                .unwrap();
                e.probe_job = Some(ProbeJob {
                    id: job.id.clone(),
                    generation: 7,
                    cancel: Arc::new(AtomicBool::new(false)),
                    receiver,
                });
            } else {
                e.stop(&mut h).unwrap();
            }
            for _ in 0..200 {
                e.tick(&mut s, &mut h, 7).unwrap();
                if !e.busy() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            e.tick(&mut s, &mut h, 7).unwrap();
            assert_eq!(
                e.controller.as_ref().unwrap().mode,
                if failed { Mode::Failed } else { Mode::Stopped }
            );
            assert_eq!(e.controller.as_ref().unwrap().model_requests, 1);
            assert!(h.unfinished_jobs().is_empty());
            assert!(
                h.pending_probe(&e.controller.as_ref().unwrap().id)
                    .is_none()
            );
            History::restore(h.document().clone()).unwrap();
        }
    }
    #[test]
    fn required_probe_observation_survives_a_full_context_budget() {
        let (mut session, _history, mut engine) = setup();
        for id in 7..31 {
            let mut record = session.records[2].clone();
            record.id = RecordId(id);
            record.question_id = Some(QuestionId::new(5).unwrap());
            record.insight.observation = "Synthetic byte observation. ".repeat(100);
            session.records.push(record);
        }
        session.validate().unwrap();
        let before = omp::context(&session, session.records.len(), RecordId(1)).unwrap();
        assert_eq!(before["record_ids"].as_array().unwrap().len(), 12);
        assert!(!before["record_ids"].as_array().unwrap().contains(&json!(7)));
        engine.controller.as_mut().unwrap().phase = Phase::NeedModel {
            observation: Some(ObservationId::new(7).unwrap()),
        };
        let context = engine.snapshot(&session).unwrap();
        let ids = context["record_ids"].as_array().unwrap();
        let evidence = context["evidence"].as_array().unwrap();
        assert_eq!(ids.len(), 12);
        assert_eq!(evidence.len(), 12);
        assert_eq!(ids.last(), Some(&json!(7)));
        assert_eq!(context["required_observation"], 7);
        assert_eq!(
            &context["observation_replaced_by_required_result"],
            before["record_ids"].as_array().unwrap().last().unwrap()
        );
        assert!(ids.iter().zip(evidence).all(|(id, item)| id == &item["id"]));
        assert!(
            evidence.last().unwrap()["observation"]
                .as_str()
                .unwrap()
                .contains("[excerpt;")
        );
        assert!(serde_json::to_vec(&context).unwrap().len() < 64000);
    }
    #[test]
    fn reader_hold_and_branch_correction_survive_live_probe_progress() {
        let (mut session, mut history, mut engine) = setup();
        let mut attention = Attention::all(session.records.len());
        attention.select(3);
        inject(
            &mut engine,
            &mut history,
            &session,
            json!(proposal(ObservationId::new(3).unwrap(), false, false)),
        );
        session
            .append_review(
                ReviewKind::Correction,
                RecordId(4),
                "Keep chosen representation separate from meaning".into(),
                "No consuming code observed".into(),
                vec![ObservationId::new(3).unwrap()],
            )
            .unwrap();
        history.capture(&session).unwrap();
        engine.tick(&mut session, &mut history, 7).unwrap();
        attention.publish(session.records.len());
        assert_eq!(attention.selected_index, 3);
        assert_eq!(attention.reading_count(), 6);
        assert!(attention.unread() > 0);
        let observed = finish_probe_paused(&mut engine, &mut history, &mut session);
        attention.publish(session.records.len());
        let snapshot = engine.snapshot(&session).unwrap();
        assert!(
            snapshot["branch_memory"]
                .to_string()
                .contains("Keep chosen representation separate from meaning")
        );
        assert_eq!(snapshot["required_observation"], json!(observed.value()));
        assert!(attention.review_next());
        assert!(matches!(
            session.records[attention.selected_index].data,
            RecordData::Review { .. }
        ));
        attention.follow();
        attention.set_hold(true);
        engine.resume(&mut history).unwrap();
        let reading = attention.selected_index;
        inject(
            &mut engine,
            &mut history,
            &session,
            json!(proposal(observed, true, true)),
        );
        engine.tick(&mut session, &mut history, 7).unwrap();
        assert!(!attention.publish(session.records.len()));
        assert_eq!(attention.selected_index, reading);
        assert!(attention.unread() > 0);
        assert_eq!(engine.controller.as_ref().unwrap().mode, Mode::Completed);
        History::restore(history.document().clone()).unwrap();
    }
}
