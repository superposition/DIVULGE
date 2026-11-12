//! Isolated OMP transport and checked next-step proposals. Models never execute probes.
use crate::domain::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub provider: String,
    pub id: String,
    pub name: String,
    pub supports_tools: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalogue {
    pub sdk_version: String,
    pub cli_version: String,
    pub selected: String,
    pub active_tools: Vec<String>,
    pub models: Vec<Model>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub v: u32,
    pub id: String,
    pub seq: u64,
    pub kind: String,
    pub data: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub question: String,
    pub why: String,
    pub method: String,
    pub enables: String,
    pub unknowns: String,
    pub evidence: Vec<RecordId>,
    pub offset: u64,
    pub length: u64,
}
impl Proposal {
    pub fn validate(&self, context: &Value) -> Result<(), String> {
        for text in [
            &self.question,
            &self.why,
            &self.method,
            &self.enables,
            &self.unknowns,
        ] {
            if text.trim().is_empty() || text.len() > 4000 {
                return Err("Proposal fields must contain 1–4000 bytes".into());
            }
        }
        let image_length = context["image_length"]
            .as_u64()
            .ok_or("No application image in this context")?;
        if !self.offset.is_multiple_of(2)
            || !self.length.is_multiple_of(2)
            || !(2..=4096).contains(&self.length)
            || self
                .offset
                .checked_add(self.length)
                .is_none_or(|end| end > image_length)
        {
            return Err("Proposed span is unaligned, too large or outside the image".into());
        }
        let supplied = context["record_ids"]
            .as_array()
            .ok_or("Missing evidence IDs")?;
        if self.evidence.is_empty()
            || self.evidence.len() > 16
            || self
                .evidence
                .iter()
                .any(|id| !supplied.contains(&json!(id.0)))
        {
            return Err("Proposal cites evidence that was not supplied".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiRun {
    pub id: String,
    pub record: RecordId,
    pub context: Value,
    pub context_sha256: String,
    pub events: Vec<Event>,
    pub proposal: Option<Proposal>,
    pub outcome: String,
    #[serde(default)]
    pub diagnostic: String,
}
impl AiRun {
    pub fn validate(&self) -> Result<(), String> {
        if self.context_sha256
            != crate::fwsc::digest(&serde_json::to_vec(&self.context).map_err(|e| e.to_string())?)
        {
            return Err("AI context identity mismatch".into());
        }
        let mut sequence = 0;
        for event in &self.events {
            check_event(event, &self.id, &mut sequence)?;
            if ["completed", "error", "cancelled"].contains(&event.kind.as_str())
                && event.seq != self.events.len() as u64
            {
                return Err("OMP transcript continues after completion".into());
            }
        }
        if let Some(proposal) = &self.proposal {
            proposal.validate(&self.context)?;
            if self.events.iter().filter(|e| e.kind == "ready").count() != 1
                || self.events.iter().filter(|e| e.kind == "proposal").count() != 1
            {
                return Err("Missing or duplicate OMP ready/proposal event".into());
            }
            if self.outcome != "completed"
                || !self.events.iter().any(|e| e.kind == "evidence_read")
                || self.events.last().is_none_or(|e| e.kind != "completed")
            {
                return Err("Proposal lacks completed evidence receipt".into());
            }
            let projected = self
                .events
                .iter()
                .find(|e| e.kind == "proposal")
                .ok_or("Missing proposal event")?;
            if serde_json::to_value(proposal).unwrap() != projected.data {
                return Err("Proposal differs from its recorded output".into());
            }
        }
        if self.events.len() > 4096 || serde_json::to_vec(self).unwrap().len() > 1048576 {
            return Err("AI run exceeds its recording limit".into());
        }
        Ok(())
    }
}
pub fn context(
    session: &Investigation,
    visible: usize,
    selected: RecordId,
) -> Result<Value, String> {
    let report = session
        .artifact
        .as_ref()
        .and_then(|a| a.firmware.as_ref())
        .and_then(|f| f.application.as_ref())
        .ok_or("Open the decoded V15 application first")?;
    let records: Vec<_> = session
        .records
        .iter()
        .take(visible)
        .filter(|r| session.is_evidence(r))
        .rev()
        .take(8)
        .collect();
    let evidence:Vec<_>=records.iter().map(|r|json!({"id":r.id.0,"title":r.title,"observation":r.insight.observation,"uncertainty":r.insight.uncertainty,"rea":session.rea_runs.iter().find(|run|run.record==r.id).and_then(|run|run.receipt.bundle().ok().flatten()).map(|bundle|bundle["records"].as_array().unwrap().iter().map(|e|json!({"evidence_id":e["evidence_id"],"provider":e["provider"],"authority":e["authority"],"file_window":e["normalized_result"]["window"],"file_entropy":e["normalized_result"]["file_entropy"],"limitations":e["limitations"]})).collect::<Vec<_>>())})).collect();
    let runs:Vec<_>=records.iter().filter_map(|r|if let RecordData::Disassembly(id)=r.data {session.disassemblies.iter().find(|run|run.id==id)}else{None}).take(2).map(|run|json!({"run":run.id.0,"scope":run.scope,"coverage":run.coverage,"instructions":run.instructions.iter().take(80).map(|i|json!({"address":i.address.0,"offset":i.span.offset.0,"width":i.width(),"text":i.text,"flow":i.flow})).collect::<Vec<_>>()})).collect();
    let attention=session.record(selected).map(|r|json!({"title":r.title,"question":r.insight.question,"analyst_note":r.note.chars().take(2000).collect::<String>()}));
    let value = json!({"objective":session.objective,"selected_record":selected.0,"attention":attention,"record_ids":records.iter().map(|r|r.id.0).collect::<Vec<_>>(),"evidence":evidence,"disassembly":runs,"image_length":report.entry().length,"mapped_base":report.mapped_base().0,"image_sha256":report.sha256,"limitations":"Static bounded evidence only. No runtime behavior or recovered function purpose established."});
    if serde_json::to_vec(&value).unwrap().len() > 64000 {
        return Err("Evidence snapshot exceeds 64 KiB".into());
    }
    Ok(value)
}
pub fn check_event(event: &Event, id: &str, sequence: &mut u64) -> Result<(), String> {
    if event.v != 1 || event.id != id || event.seq != *sequence + 1 {
        return Err("OMP protocol version, request ID or sequence mismatch".into());
    }
    if ![
        "ready",
        "draft",
        "evidence_read",
        "proposal",
        "completed",
        "error",
        "cancelled",
        "cancelling",
    ]
    .contains(&event.kind.as_str())
    {
        return Err("Unknown OMP event".into());
    }
    if event.kind == "ready" {
        let catalogue: Catalogue =
            serde_json::from_value(event.data.clone()).map_err(|e| e.to_string())?;
        if catalogue.active_tools != ["read_evidence"] {
            return Err("OMP active tools exceed the allowed evidence reader".into());
        }
    }
    *sequence = event.seq;
    Ok(())
}
pub struct Job {
    pub receiver: mpsc::Receiver<Result<Event, String>>,
    pub cancel: Arc<AtomicBool>,
    pub id: String,
    pub context: Value,
    pub events: Vec<Event>,
    pub generation: u64,
    pub propose: bool,
}
impl Drop for Job {
    fn drop(&mut self) {
        if self
            .events
            .last()
            .is_none_or(|event| !["completed", "error", "cancelled"].contains(&event.kind.as_str()))
        {
            self.cancel.store(true, Ordering::Relaxed);
        }
    }
}
pub fn start(context: Value, model: String, propose: bool, generation: u64) -> Job {
    let id = format!(
        "omp-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let request = json!({"v":1,"id":id,"op":if propose{"propose"}else{"discover"},"model":model,"context":context});
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let (send, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        if let Err(error) = transport(request, worker_cancel, &send) {
            let _ = send.send(Err(error));
        }
    });
    Job {
        receiver,
        cancel,
        id,
        context,
        events: vec![],
        generation,
        propose,
    }
}
fn transport(
    request: Value,
    cancel: Arc<AtomicBool>,
    send: &mpsc::Sender<Result<Event, String>>,
) -> Result<(), String> {
    let root = std::env::temp_dir().join(request["id"].as_str().unwrap());
    std::fs::create_dir(&root).map_err(|e| e.to_string())?;
    let result = (|| {
        std::fs::write(
            root.join("bridge.ts"),
            include_str!("../adapters/omp_bridge.ts"),
        )
        .map_err(|e| e.to_string())?;
        let bun = std::env::var_os("DIVULGE_BUN")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                    .unwrap_or_default();
                let bundled = std::path::PathBuf::from(home).join(if cfg!(windows) {
                    ".bun/bin/bun.exe"
                } else {
                    ".bun/bin/bun"
                });
                if bundled.is_file() {
                    bundled
                } else {
                    "bun".into()
                }
            });
        let mut command = Command::new(bun);
        command
            .arg(root.join("bridge.ts"))
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("Could not start Bun/OMP: {e}"))?;
        let mut stdin = child.stdin.take().unwrap();
        if let Err(e) = writeln!(stdin, "{request}") {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e.to_string());
        }
        let stdout = child.stdout.take().unwrap();
        let (lines_tx, lines_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut total = 0;
            loop {
                let mut line = Vec::new();
                match reader.by_ref().take(262145).read_until(b'\n', &mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        total += line.len();
                        if line.len() > 262144 || total > 1048576 {
                            let _ = lines_tx
                                .send(Err("OMP output exceeded 1 MiB/256 KiB line limit".into()));
                            break;
                        }
                        let event = serde_json::from_slice::<Event>(&line)
                            .map_err(|_| "OMP emitted an invalid protocol line".to_string());
                        if lines_tx.send(event).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = lines_tx.send(Err(e.to_string()));
                        break;
                    }
                }
            }
        });
        let started = Instant::now();
        let mut stopping = None;
        let mut sequence = 0;
        let mut terminal = false;
        let mut terminal_at = None;
        let result = loop {
            if terminal_at.is_some_and(|at: Instant| at.elapsed() > Duration::from_secs(3)) {
                break Ok(());
            }
            if !terminal
                && stopping.is_none()
                && (cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(100))
            {
                let _ = writeln!(stdin, "{}", json!({"v":1,"id":request["id"],"op":"cancel"}));
                stopping = Some(Instant::now());
            }
            if stopping.is_some_and(|s| s.elapsed() > Duration::from_secs(3)) {
                break Err(
                    "OMP stopped after cancellation/deadline; partial transcript retained".into(),
                );
            }
            match lines_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(event)) => {
                    if let Err(e) =
                        check_event(&event, request["id"].as_str().unwrap(), &mut sequence)
                    {
                        break Err(e);
                    }
                    if terminal {
                        break Err("OMP emitted output after terminal result".into());
                    }
                    terminal = ["completed", "error", "cancelled"].contains(&event.kind.as_str());
                    if terminal {
                        terminal_at = Some(Instant::now());
                        stopping = None;
                    }
                    let _ = send.send(Ok(event));
                }
                Ok(Err(e)) => break Err(e),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break if terminal {
                        Ok(())
                    } else {
                        Err("OMP ended before completion".into())
                    };
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        };
        let _ = child.kill();
        let _ = child.wait();
        result
    })();
    // This unique directory was created above and is used only by this owned bridge.
    let _ = std::fs::remove_dir_all(&root);
    result
}
impl Investigation {
    pub fn append_ai_run(&mut self, mut run: AiRun) -> Result<usize, String> {
        if self.ai_runs.len() >= 16 || self.records.len() >= 200 {
            return Err("Investigation AI run limit reached".into());
        }
        run.record = RecordId(
            self.records
                .iter()
                .map(|r| r.id.0)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or("Record ID overflow")?,
        );
        run.validate()?;
        let (title, insight, plan) = if let Some(p) = &run.proposal {
            ("AI proposal / next span".into(),HarnessInsight{question:p.question.clone(),purpose:p.why.clone(),action:format!("Proposed only: app.bin+0x{:X}, {} bytes. {}",p.offset,p.length,p.method),observation:"OMP read the supplied evidence snapshot; this proposed probe has not run.".into(),explanation:p.enables.clone(),uncertainty:p.unknowns.clone(),next_step:"Stage this span in Code, review it, then Disassemble selection.".into()},Some(QuestionPlan{why:p.why.clone(),steps:vec![p.method.clone()],enables:p.enables.clone(),answered_when:"The proposed span has been inspected and its actual output compared with the question.".into(),if_not:p.unknowns.clone()}))
        } else {
            (
                "AI request / incomplete".into(),
                HarnessInsight {
                    question: "What should we inspect next?".into(),
                    purpose: "Find an evidence-linked next step without running a probe.".into(),
                    action: "Request one proposal from an isolated OMP session using the bounded evidence snapshot.".into(),
                    observation: format!(
                        "OMP outcome: {}. Partial transcript retained; no validated proposal.",
                        run.outcome
                    ),
                    uncertainty: "A draft is not a completed proposal.".into(),
                    explanation: "This request did not produce a validated next-step proposal.".into(),
                    next_step: "Review the receipt and select an available model before retrying.".into(),
                },
                None,
            )
        };
        let index = self.records.len();
        self.records.push(Record {
            id: run.record,
            title,
            summary: format!("OMP / {} / not executed", run.outcome),
            data: RecordData::Unknown {
                next_probe: "Review the model receipt and proposed scope".into(),
            },
            insight,
            note: String::new(),
            question_id: None,
            question_plan: plan,
        });
        if let Some(p) = &run.proposal {
            for id in &p.evidence {
                self.edges.push(Edge {
                    from: *id,
                    to: run.record,
                    relation: Relation::Suggests,
                });
            }
        }
        self.ai_runs.push(run);
        if let Err(e) = self.validate() {
            let id = self.records.pop().unwrap().id;
            self.edges.retain(|edge| edge.to != id);
            self.ai_runs.pop();
            return Err(e);
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recorded_proposals_cannot_outlive_or_change_their_receipt() {
        let ctx = json!({"record_ids":[3],"image_length":512});
        let proposal = Proposal {
            question: "Where?".into(),
            why: "Find target".into(),
            method: "Decode bytes".into(),
            enables: "Compare".into(),
            unknowns: "Execution".into(),
            evidence: vec![RecordId(3)],
            offset: 4,
            length: 16,
        };
        let events = vec![
            Event {
                v: 1,
                id: "a".into(),
                seq: 1,
                kind: "ready".into(),
                data: json!({"sdk_version":"test","cli_version":"test","selected":"p/m","models":[],"active_tools":["read_evidence"]}),
            },
            Event {
                v: 1,
                id: "a".into(),
                seq: 2,
                kind: "evidence_read".into(),
                data: json!({"record_ids":[3]}),
            },
            Event {
                v: 1,
                id: "a".into(),
                seq: 3,
                kind: "proposal".into(),
                data: serde_json::to_value(&proposal).unwrap(),
            },
            Event {
                v: 1,
                id: "a".into(),
                seq: 4,
                kind: "completed".into(),
                data: Value::Null,
            },
        ];
        let mut run = AiRun {
            id: "a".into(),
            record: RecordId(1),
            context_sha256: crate::fwsc::digest(&serde_json::to_vec(&ctx).unwrap()),
            context: ctx,
            events,
            proposal: Some(proposal),
            outcome: "completed".into(),
            diagnostic: String::new(),
        };
        run.validate().unwrap();
        let restored: AiRun = serde_json::from_slice(&serde_json::to_vec(&run).unwrap()).unwrap();
        restored.validate().unwrap();
        run.proposal.as_mut().unwrap().offset = 8;
        assert!(run.validate().is_err());
        run.proposal.as_mut().unwrap().offset = 4;
        run.events.pop();
        assert!(run.validate().is_err());
    }
    #[test]
    #[ignore = "requires installed Bun and OMP SDK; no model request"]
    fn bridge_cancellation_is_correlated_and_terminal() {
        let job = start(Value::Null, String::new(), false, 0);
        job.cancel.store(true, Ordering::Relaxed);
        let mut sequence = 0;
        let started = Instant::now();
        loop {
            let event = job
                .receiver
                .recv_timeout(Duration::from_secs(30))
                .unwrap()
                .unwrap();
            check_event(&event, &job.id, &mut sequence).unwrap();
            assert_ne!(event.kind, "completed");
            if event.kind == "cancelled" {
                break;
            }
            assert!(started.elapsed() < Duration::from_secs(30));
        }
    }
    #[test]
    fn proposals_require_supplied_evidence_and_checked_scope() {
        let ctx = json!({"record_ids":[3],"image_length":512});
        let mut p = Proposal {
            question: "Where?".into(),
            why: "Find a branch".into(),
            method: "Decode".into(),
            enables: "Inspect target".into(),
            unknowns: "Runtime".into(),
            evidence: vec![RecordId(3)],
            offset: 0,
            length: 256,
        };
        assert!(p.validate(&ctx).is_ok());
        p.evidence = vec![RecordId(99)];
        assert!(p.validate(&ctx).is_err());
        p.evidence = vec![RecordId(3)];
        p.offset = u64::MAX;
        assert!(p.validate(&ctx).is_err());
        p.offset = 1;
        assert!(p.validate(&ctx).is_err());
        p.offset = 510;
        assert!(p.validate(&ctx).is_err());
    }
    #[test]
    fn protocol_rejects_stale_out_of_order_and_ambient_tools() {
        let mut seq = 0;
        let mut e = Event {
            v: 1,
            id: "a".into(),
            seq: 1,
            kind: "draft".into(),
            data: json!("draft"),
        };
        assert!(check_event(&e, "b", &mut seq).is_err());
        assert!(check_event(&e, "a", &mut seq).is_ok());
        assert!(check_event(&e, "a", &mut seq).is_err());
        e.seq = 2;
        e.kind = "ready".into();
        e.data = json!({"sdk_version":"x","cli_version":"x","selected":"p/m","models":[],"active_tools":["bash"]});
        assert!(check_event(&e, "a", &mut seq).is_err());
    }
}
