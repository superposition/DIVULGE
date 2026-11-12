use divulge::{
    domain::*,
    rea::{self, Config, Job, ReaRun, Request},
};
use eframe::egui::{self, RichText};
use std::{path::Path, sync::atomic::Ordering};
pub struct ReaView {
    pub open: bool,
    pub job: Option<Job>,
    config: Config,
    status: String,
    ready: bool,
    offset: String,
    length: u64,
    block_size: u64,
    latest: Option<usize>,
}
impl Default for ReaView {
    fn default() -> Self {
        Self {
            open: false,
            job: None,
            config: Config::discover(),
            status: String::new(),
            ready: false,
            offset: "0x0".into(),
            length: 4096,
            block_size: 4096,
            latest: None,
        }
    }
}
impl ReaView {
    pub fn reset(&mut self) {
        self.latest = None;
        self.status.clear();
    }
    pub fn poll(&mut self, session: &mut Investigation, generation: u64) -> bool {
        let Some(job) = &self.job else { return false };
        let mut receipt = match job.receiver.try_recv() {
            Ok(value) => value,
            Err(std::sync::mpsc::TryRecvError::Empty) => return false,
            Err(_) => {
                self.status = "REA worker disconnected; no evidence published".into();
                self.job = None;
                return false;
            }
        };
        let job = self.job.take().unwrap();
        if job.cancel.load(Ordering::Relaxed) {
            receipt.kind = "cancelled".into();
            receipt.bundle_json = None;
            receipt.semantic.clear();
            receipt.error = "Completion arrived after cancellation; no evidence published.".into();
        }
        if job.generation != generation {
            return false;
        }
        if matches!(job.request, Request::Discover) {
            self.ready = receipt.kind == "ready" && receipt.rea_version == "6.8.0";
            self.status = if self.ready {
                format!(
                    "REA {} / {}",
                    receipt.rea_version,
                    receipt.capabilities.join(", ")
                )
            } else {
                receipt.error
            };
            return false;
        }
        self.status = format!("{} / {}", receipt.kind, receipt.error);
        let run = ReaRun {
            record: RecordId(0),
            request: job.request.clone(),
            receipt,
        };
        match session.append_rea(run) {
            Ok(index) => {
                self.latest = Some(index);
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
        source: Option<&Path>,
        generation: u64,
        busy: bool,
    ) -> Option<usize> {
        let mut review = None;
        let mut open = self.open;
        egui::Window::new("REA / Original-file evidence").open(&mut open).default_width(620.0).show(ctx,|ui|{
            ui.label("Read byte distribution in the original artifact. No firmware execution, extraction or disassembly occurs here.");
            let running=self.job.is_some();let available=!running&&!busy;
            ui.collapsing("Installed REA / prerequisites",|ui|{ui.label("Node executable (REA-compatible runtime)");ui.text_edit_singleline(&mut self.config.node);ui.label("rea-agents package directory");ui.text_edit_singleline(&mut self.config.package);ui.label("Validated adapter: REA 6.8.0. No setup or repair is performed.");});
            if ui.add_enabled(available,egui::Button::new("Detect REA capabilities")).clicked(){self.job=Some(rea::start(self.config.clone(),Request::Discover,generation));self.status="Checking local REA...".into();}
            ui.label(&self.status);
            ui.separator();ui.label(RichText::new("Scope is file bytes, not app.bin offsets or mapped addresses.").strong());
            ui.horizontal(|ui|{ui.label("file+");ui.text_edit_singleline(&mut self.offset);});
            ui.horizontal(|ui|{ui.label("Bytes");ui.add(egui::DragValue::new(&mut self.length).range(0..=1024*1024));ui.label("Block size");egui::ComboBox::from_id_salt("rea_block").selected_text(self.block_size.to_string()).show_ui(ui,|ui|{for size in [256,512,1024,2048,4096,8192,16384,32768,65536]{ui.selectable_value(&mut self.block_size,size,size.to_string());}});});
            if source.is_none(){ui.label("Reverify the original file through File > Verify original source before a live run.");}
            let offset=self.offset.trim().strip_prefix("0x").map_or_else(||self.offset.trim().parse(),|s|u64::from_str_radix(s,16));
            let request=source.zip(session.artifact.as_ref()).ok_or("No freshly attached source".into()).and_then(|(path,artifact)|offset.map_err(|_|"Invalid file offset".to_string()).and_then(|offset|Request::entropy(path.to_path_buf(),artifact,offset,self.length,self.block_size)));
            if let Err(e)=&request{ui.label(e);}
            let capacity=session.rea_runs.len()<16&&session.records.len()<200;
            if ui.add_enabled(available&&self.ready&&capacity&&request.is_ok(),egui::Button::new("Inspect byte entropy")).clicked(){self.job=Some(rea::start(self.config.clone(),request.unwrap(),generation));self.status="REA is reading and hashing the original artifact...".into();}
            if ui.add_enabled(available&&self.ready&&capacity&&session.artifact.is_some(),egui::Button::new("Import canonical entropy bundle...")).clicked()&&let Some(path)=rfd::FileDialog::new().add_filter("REA evidence bundle",&["json"]).pick_file(){self.job=Some(rea::start(self.config.clone(),Request::ImportBundle{path,artifact_sha256:session.artifact.as_ref().unwrap().sha256.clone()},generation));}
            if running{ui.spinner();if ui.button("Cancel REA request").clicked(){self.job.as_ref().unwrap().cancel.store(true,Ordering::Relaxed);}}
            if let Some(index)=self.latest&&index<session.records.len()&&ui.button("Review recorded result").clicked(){review=Some(index);}
        });
        self.open = open;
        if review.is_some() {
            self.open = false;
        }
        review
    }
}

pub fn receipt(ui: &mut egui::Ui, run: &ReaRun) {
    ui.label(RichText::new("@ REA / canonical evidence").color(crate::graph::ACCENT));
    ui.label(format!(
        "{} / REA {} / {}",
        run.receipt.id, run.receipt.rea_version, run.receipt.kind
    ));
    if !run.receipt.error.is_empty() {
        ui.label(&run.receipt.error);
    }
    if !run.receipt.partial_output.is_empty() {
        ui.collapsing(
            "Partial transport output / not evidence / first 64 KiB",
            |ui| {
                ui.add(egui::Label::new(&run.receipt.partial_output).wrap());
            },
        );
    }
    if let Ok(Some(bundle)) = run.receipt.bundle() {
        for evidence in bundle["records"].as_array().unwrap() {
            ui.label(
                evidence["evidence_id"]
                    .as_str()
                    .unwrap_or("Invalid identity"),
            );
            ui.label(format!(
                "provider {} / authority {} / {}",
                evidence["provider"]["id"], evidence["authority"], evidence["confidence"]
            ));
            let result = &evidence["normalized_result"];
            ui.label(format!(
                "file+0x{:X} / {} B / block {} B",
                result["window"]["offset"].as_u64().unwrap_or(0),
                result["window"]["length"],
                result["block_size"]
            ));
            ui.label(format!(
                "Whole-file entropy: {} bits/byte",
                result["file_entropy"]
            ));
            ui.label("Selected block observations / file offset / bytes / entropy");
            for block in result["blocks"].as_array().unwrap().iter().take(64) {
                ui.label(format!(
                    "0x{:08X} / {} / {}",
                    block["offset"].as_u64().unwrap_or(0),
                    block["length"],
                    block["entropy"]
                ));
            }
            if result["blocks"].as_array().unwrap().len() > 64 {
                ui.label("Showing first 64 blocks; complete data retained below.");
            }
            ui.label(if result["runs"].as_array().unwrap().is_empty() {
                "Classified runs: examined, none at REA's thresholds"
            } else {
                "Classified runs retained in normalized result"
            });
            ui.label(
                "Relationship coverage: unavailable (REA null); no inferred graph destinations",
            );
            for limit in evidence["limitations"].as_array().unwrap() {
                ui.label(format!("! {}", limit.as_str().unwrap_or("")));
            }
            for link in evidence["evidence_links"].as_array().unwrap() {
                let present = bundle["records"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["evidence_id"] == *link);
                ui.label(format!(
                    "Evidence link {link} / {}",
                    if present {
                        "in this bundle"
                    } else {
                        "unresolved / record not imported"
                    }
                ));
            }
        }
        ui.collapsing(
            "Complete original REA bundle / includes local paths",
            |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(serde_json::to_string_pretty(&bundle).unwrap()).monospace(),
                    )
                    .wrap(),
                );
            },
        );
    } else {
        ui.label("Evidence unavailable. This is not an examined-empty result.");
    }
}
