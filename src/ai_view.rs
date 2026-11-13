use crate::graph::{ACCENT, MUTED};
use divulge::{
    domain::*,
    omp::{self, AiRun, Catalogue, Job, Proposal},
};
use eframe::egui::{self, RichText};
use std::sync::atomic::Ordering;

#[derive(Default)]
pub struct AiView {
    pub open: bool,
    pub job: Option<Job>,
    pub catalogue: Option<Catalogue>,
    pub model: String,
    status: String,
    draft: String,
    search: String,
    pub latest: Option<usize>,
}
#[derive(Default)]
pub struct Action {
    pub review: Option<usize>,
    pub stage: Option<(u64, u64)>,
}
impl AiView {
    pub fn reset(&mut self) {
        self.open = false;
        self.latest = None;
        self.draft.clear();
        self.status.clear();
        self.search.clear();
    }
    pub fn poll(&mut self, session: &mut Investigation, generation: u64) -> bool {
        let mut finished = None;
        if let Some(job) = &mut self.job {
            loop {
                match job.receiver.try_recv() {
                    Ok(Ok(event)) => {
                        match event.kind.as_str() {
                            "ready" => {
                                if let Ok(c) =
                                    serde_json::from_value::<Catalogue>(event.data.clone())
                                {
                                    if self.model.is_empty() {
                                        self.model = c.selected.clone();
                                    }
                                    self.catalogue = Some(c);
                                    self.status = "OMP ready / evidence reader only".into();
                                }
                            }
                            "evidence_read" => {
                                self.status =
                                    "Model read the supplied evidence; composing a proposal..."
                                        .into()
                            }
                            "draft" => {
                                if let Some(text) = event.data.as_str()
                                    && self.draft.len() + text.len() < 65536
                                {
                                    self.draft.push_str(text);
                                }
                            }
                            "completed" | "error" | "cancelled" => {
                                finished = Some(event.kind.clone())
                            }
                            _ => {}
                        }
                        job.events.push(event);
                        if finished.is_some() {
                            break;
                        }
                    }
                    Ok(Err(error)) => {
                        self.status = error;
                        finished = Some("transport_error".into());
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        finished = Some("disconnected".into());
                        break;
                    }
                }
            }
        }
        let Some(mut outcome) = finished else {
            return false;
        };
        let job = self.job.take().unwrap();
        if job.generation != generation {
            return false;
        }
        if !job.propose {
            self.status = if outcome == "completed" {
                "Model catalogue loaded. Choose a model or keep OMP's default.".into()
            } else {
                format!(
                    "OMP discovery {outcome}: {}",
                    job.events
                        .last()
                        .map(|e| e.data.to_string())
                        .unwrap_or_else(|| self.status.clone())
                )
            };
            return false;
        }
        if job.cancel.load(Ordering::Relaxed) {
            outcome = "cancelled".into();
        }
        let proposal = if outcome == "completed" {
            job.events
                .iter()
                .find(|e| e.kind == "proposal")
                .and_then(|e| serde_json::from_value::<Proposal>(e.data.clone()).ok())
                .filter(|p| p.validate(&job.context).is_ok())
        } else {
            None
        };
        if outcome == "completed" && proposal.is_none() {
            outcome = "invalid_proposal".into();
        }
        let run = AiRun {
            analysis_record: None,
            id: job.id.clone(),
            record: RecordId(0),
            context: job.context.clone(),
            context_sha256: divulge::fwsc::digest(&serde_json::to_vec(&job.context).unwrap()),
            events: job.events.clone(),
            proposal,
            outcome: outcome.clone(),
            diagnostic: if outcome == "transport_error" {
                self.status.clone()
            } else {
                job.events
                    .iter()
                    .find(|e| e.kind == "error")
                    .map(|e| e.data.to_string())
                    .unwrap_or_default()
            },
        };
        match session.append_ai_run(run) {
            Ok(index) => {
                self.latest = Some(index);
                self.status = format!(
                    "{outcome} / recorded in the investigation. Proposed probes have not run."
                );
                true
            }
            Err(error) => {
                self.status = error;
                false
            }
        }
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        session: &Investigation,
        visible: usize,
        selected: RecordId,
        generation: u64,
        busy: bool,
    ) -> Action {
        let mut action = Action::default();
        let available: Vec<usize> = session
            .records
            .iter()
            .take(visible)
            .enumerate()
            .filter(|(_, record)| session.ai_runs.iter().any(|run| run.record == record.id))
            .map(|(index, _)| index)
            .collect();
        if self.latest.is_none_or(|index| !available.contains(&index)) {
            self.latest = available.last().copied();
        }
        let mut open = self.open;
        egui::Window::new("OMP / Next investigation step").open(&mut open).default_width(620.0).default_height(600.0).resizable(true).show(ctx,|ui|{
            ui.label(RichText::new("Ask a question. Read the evidence. Propose a bounded probe.").color(ACCENT));
            ui.label("OMP receives durable branch context, up to 12 evidence summaries and 2 instruction excerpts. Corrections, counter-evidence and omitted context are identified. The selected provider processes this snapshot. Credentials stay with OMP.");
            let running=self.job.is_some();
            if ui.add_enabled(!running&&!busy,egui::Button::new("Connect / refresh models")).clicked(){self.job=Some(omp::start(serde_json::Value::Null,String::new(),false,generation));self.status="Discovering installed OMP models...".into();}
            if let Some(c)=&self.catalogue {
                ui.label(RichText::new(format!("SDK {} / CLI {} / active tool: read_evidence",c.sdk_version,c.cli_version)).monospace().small());
                if c.cli_version.trim_start_matches("omp/")!=c.sdk_version{ui.label(RichText::new("Version skew detected; the active tool check passed for this SDK session.").small().color(MUTED));}
                ui.horizontal(|ui|{ui.label("Filter models");ui.text_edit_singleline(&mut self.search);});
                egui::ComboBox::from_id_salt("omp_model").width(570.0).selected_text(&self.model).show_ui(ui,|ui|{
                    for m in &c.models {let id=format!("{}/{}",m.provider,m.id);if m.supports_tools&&id.to_lowercase().contains(&self.search.to_lowercase()){ui.selectable_value(&mut self.model,id.clone(),id);}}
                });
            }
            let snapshot=omp::context(session,visible,selected);
            ui.horizontal(|ui|{
                if ui.add_enabled(!running&&!busy&&self.catalogue.is_some()&&snapshot.is_ok()&&session.ai_runs.len()<16&&session.records.len()<200,egui::Button::new("Propose next step")).clicked(){
                    self.draft.clear();self.latest=None;self.job=Some(omp::start(snapshot.clone().unwrap(),self.model.clone(),true,generation));self.status="Starting isolated OMP request...".into();
                }
                if running {ui.spinner();if ui.button("Cancel request").clicked(){self.job.as_ref().unwrap().cancel.store(true,Ordering::Relaxed);self.status="Cancelling; retaining partial transcript...".into();}}
            });
            ui.label(&self.status);
            if let Err(error)=&snapshot{ui.label(error);}
            ui.separator();
            if !available.is_empty(){egui::ComboBox::from_id_salt("omp_recorded_request").selected_text(self.latest.map_or_else(||"Recorded requests".into(),|index|format!("Recorded request / #{}",session.records[index].id.0))).show_ui(ui,|ui|{for index in &available{ui.selectable_value(&mut self.latest,Some(*index),format!("#{} / {}",session.records[*index].id.0,session.records[*index].summary));}});}
            egui::ScrollArea::vertical().max_height(420.0).show(ui,|ui|{
                if let Some(index)=self.latest && let Some(run)=session.ai_runs.iter().find(|r|session.records[index].id==r.record){
                    if let Some(p)=&run.proposal{
                        ui.heading(&p.question);
                        for (label,value) in [("Why",&p.why),("How",&p.method),("To what end",&p.enables),("Still unknown",&p.unknowns)]{ui.label(RichText::new(label).color(ACCENT));ui.label(value);}
                        ui.label(RichText::new(format!("PROPOSED / app.bin+0x{:X} / {} bytes",p.offset,p.length)).monospace());
                        ui.horizontal(|ui|{
                            if ui.button("Review in graph").clicked(){action.review=Some(index);}
                            if ui.add_enabled(p.operation == omp::Operation::Disassemble, egui::Button::new("Stage span in Code")).clicked(){action.stage=Some((p.offset,p.length));}
                        });
                    }else{ui.label("No validated proposal. Review the recorded receipt for the outcome.");if ui.button("Review receipt").clicked(){action.review=Some(index);}}
                }
                ui.collapsing("Provisional text / never executed",|ui|{ui.add(egui::Label::new(RichText::new(&self.draft).monospace()).wrap());});
                ui.collapsing("Snapshot sent on next request",|ui|{if let Ok(value)=snapshot{ui.add(egui::Label::new(RichText::new(serde_json::to_string_pretty(&value).unwrap()).monospace().small()).wrap());}});
            });
        });
        self.open = open;
        if action.review.is_some() || action.stage.is_some() {
            self.open = false;
        }
        action
    }
}
