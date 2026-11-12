use divulge::{
    domain::*,
    history::{History, ReviewKind},
};
use eframe::egui::{self, RichText};

#[derive(Default)]
pub struct HistoryView {
    pub open: bool,
    pub export_open: bool,
    frontier: usize,
    preview: Option<Investigation>,
    kind: ReviewKind,
    rationale: String,
    limitation: String,
    evidence: Vec<ObservationId>,
    export_options: divulge::export::Options,
    message: String,
    export_message: String,
}
impl HistoryView {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        history: &History,
        session: &mut Investigation,
        selected: RecordId,
    ) -> Option<usize> {
        let mut created = None;
        let mut open = self.open;
        egui::Window::new("Case history / offline replay and review").open(&mut open).default_width(700.0).max_height(560.0).vscroll(true).show(ctx,|ui|{
            ui.label(&history.document().baseline_provenance);
            ui.label(format!("{} ordered events / {} records",history.events().len(),session.records.len()));
            if !history.unfinished_jobs().is_empty(){ui.label("A job start without an end does not establish completion. Reopening never resumes saved jobs.");}
            egui::ScrollArea::vertical().id_salt("history_events").max_height(190.0).show(ui,|ui|{
                if ui.selectable_label(self.frontier==0,"0000 / starting snapshot").clicked(){self.frontier=0;self.preview=history.replay(0).ok();}
                for event in history.events(){if ui.selectable_label(self.frontier==event.sequence as usize,format!("{:04} / {}",event.sequence,event.summary())).clicked(){self.frontier=event.sequence as usize;self.preview=history.replay(self.frontier).ok();}}
            });
            if let Some(snapshot)=&self.preview{
                ui.collapsing(format!("Reconstructed checkpoint {} / read only / {} records",self.frontier,snapshot.records.len()),|ui|{
                    egui::ScrollArea::vertical().id_salt("history_preview").max_height(200.0).show(ui,|ui|{
                        ui.label(&snapshot.objective);
                        for record in &snapshot.records{ui.label(format!("#{} / {}",record.id.0,record.title));if !record.note.is_empty(){ui.label(RichText::new(&record.note).monospace());}}
                    });
                });
            }
            ui.separator();ui.label(format!("Append review of selected record #{}",selected.0));
            ui.horizontal(|ui|{for kind in [ReviewKind::Steering,ReviewKind::Correction,ReviewKind::Resolution]{ui.selectable_value(&mut self.kind,kind,format!("{kind:?}"));}});
            ui.label("Rationale / what changes and why");ui.add(egui::TextEdit::multiline(&mut self.rationale).desired_rows(2).desired_width(f32::INFINITY));
            ui.label("Limits / what this does not establish");ui.add(egui::TextEdit::multiline(&mut self.limitation).desired_rows(2).desired_width(f32::INFINITY));
            ui.collapsing(format!("Supporting evidence / {} selected",self.evidence.len()),|ui|{
                egui::ScrollArea::vertical().id_salt("review_evidence").max_height(125.0).show(ui,|ui|{for record in session.records.iter().filter(|r|session.is_evidence(r)){
                    let id=ObservationId::new(record.id.0).unwrap();let mut checked=self.evidence.contains(&id);
                    if ui.checkbox(&mut checked,format!("#{} / {}",record.id.0,record.title)).changed(){if checked{self.evidence.push(id);}else{self.evidence.retain(|v|*v!=id);}}
                }});
            });
            ui.label("Resolutions require evidence. The original finding remains intact; this records a scoped analyst decision.");
            if ui.add_enabled(!self.rationale.trim().is_empty()&&!self.limitation.trim().is_empty(),egui::Button::new("Append review record")).clicked(){
                match session.append_review(self.kind,selected,self.rationale.clone(),self.limitation.clone(),self.evidence.clone()){
                    Ok(index)=>{created=Some(index);self.rationale.clear();self.limitation.clear();self.evidence.clear();self.message="Review appended; save to persist the history.".into();}
                    Err(e)=>self.message=e,
                }
            }
            ui.label(&self.message);
        });
        self.open = open;
        let mut export_open = self.export_open;
        egui::Window::new("Export portable review").open(&mut export_open).default_width(580.0).max_height(560.0).vscroll(true).show(ctx,|ui|{
            ui.label("Exports a review projection, separate from the complete local replay case. Default output omits byte contents, analyst text, machine paths and model transcripts.");
            ui.checkbox(&mut self.export_options.include_bytes,"Include retained byte contents");
            ui.checkbox(&mut self.export_options.include_analyst_text,"Include titles, explanations and analyst notes (review before sharing)");
            if !self.export_options.include_bytes||!self.export_options.include_analyst_text{self.export_options.include_original_receipts=false;}
            ui.add_enabled_ui(self.export_options.include_bytes&&self.export_options.include_analyst_text,|ui|{ui.checkbox(&mut self.export_options.include_original_receipts,"Include original receipts: local paths, model snapshots, transcripts and bytes");});
            ui.label("Credential stores are never exported. Free text and original tool receipts may contain private information; inspect the preview before sharing.");
            match divulge::export::review(session,self.export_options).and_then(|v|serde_json::to_string_pretty(&v).map_err(|e|e.to_string())){
                Ok(json)=>{
                    ui.collapsing(format!("Exact export preview / {} bytes",json.len()),|ui|{egui::ScrollArea::vertical().max_height(250.0).show(ui,|ui|{ui.add(egui::Label::new(RichText::new(&json).monospace().small()).wrap());});});
                    if ui.button("Save review export...").clicked()&&let Some(path)=rfd::FileDialog::new().set_file_name("review.divulge-export.json").add_filter("JSON review",&["json"]).save_file(){self.export_message=match divulge::storage::atomic_write(&path,json.as_bytes()){Ok(())=>"Review export saved. The current replay case is unchanged.".into(),Err(e)=>e};}
                }
                Err(e)=>{ui.label(e);}
            }ui.label(&self.export_message);
        });
        self.export_open = export_open;
        created
    }
}
