use crate::graph::{ACCENT, MUTED};
use divulge::{
    domain::*,
    engine::{Engine, Settings, Target},
    execution::{Mode, Phase},
    omp::Catalogue,
};
use eframe::egui::{self, RichText};

#[derive(Default)]
pub enum Action {
    #[default]
    None,
    Models,
    Start(Settings),
    Pause,
    Resume,
    Stop,
}
pub struct EngineView {
    pub open: bool,
    question: Option<QuestionId>,
    objective: String,
    offset: String,
    length: u64,
    budget: u32,
}
impl Default for EngineView {
    fn default() -> Self {
        Self {
            open: false,
            question: None,
            objective: String::new(),
            offset: "0x0".into(),
            length: 256,
            budget: 12,
        }
    }
}
impl EngineView {
    pub fn prepare(&mut self, session: &Investigation, selected: RecordId) {
        if self.question.is_none() {
            self.question = session
                .question_for(selected)
                .and_then(|r| QuestionId::new(r.id.0).ok());
            self.objective = session.objective.clone();
            if session.origin == Origin::WorkshopFixture {
                self.length = 2;
            }
        }
        self.open = true;
    }
    pub fn controls(ui: &mut egui::Ui, engine: &Engine) -> Action {
        let mut action = Action::None;
        if let Some(c) = &engine.controller {
            ui.horizontal_wrapped(|ui| {
                ui.set_min_height(24.0);
                ui.label(
                    RichText::new(format!("{:?} / {}/{} probes", c.mode, c.used, c.budget))
                        .monospace()
                        .color(ACCENT),
                );
                if c.mode == Mode::Running && ui.button("Pause AI").clicked() {
                    action = Action::Pause;
                }
                if c.mode == Mode::Paused && ui.button("Resume AI").clicked() {
                    action = Action::Resume;
                }
                if matches!(c.mode, Mode::Running | Mode::Paused) && ui.button("Stop AI").clicked()
                {
                    action = Action::Stop;
                }
            });
            ui.add(egui::Label::new(&c.status).truncate())
                .on_hover_text(&c.status);
        }
        action
    }
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        session: &Investigation,
        engine: &Engine,
        target: &Result<Target, String>,
        model: &mut String,
        catalogue: Option<&Catalogue>,
        busy: bool,
    ) -> Action {
        let mut action = Action::None;
        let mut open = self.open;
        egui::Window::new("AI / Bounded investigation").open(&mut open).default_width(600.0).max_height(550.0).vscroll(true).show(ctx, |ui| {
            ui.label(RichText::new("Question > checked proposal > probe > observation > explanation").color(ACCENT));
            let controls = Self::controls(ui, engine);
            if !matches!(controls, Action::None) { action = controls; }
            if let Some(c) = &engine.controller {
                let phase = match c.phase { Phase::NeedModel{..} => "Explanation / next proposal", Phase::AwaitingModel{..} => "Waiting for OMP", Phase::ReadyProbe{..} => "Proposal checked", Phase::AwaitingProbe{..} => "One probe executing", Phase::Done => "Cycle ended" };
                ui.monospace(format!("{phase} / question #{} / {} model request(s)", c.question.value(), c.model_requests));
            }
            ui.separator();
            ui.add_enabled_ui(!engine.busy() && !busy, |ui| {
                if ui.button("Configure / refresh OMP models...").clicked() { action = Action::Models; }
                ui.label("Model / provider receiving the evidence snapshot");
                if let Some(c) = catalogue {
                    egui::ComboBox::from_id_salt("cycle_model").selected_text(model.as_str()).width(540.0).show_ui(ui, |ui| {
                        for m in &c.models {
                            if m.supports_tools { let id = format!("{}/{}", m.provider, m.id); ui.selectable_value(model, id.clone(), id); }
                        }
                    });
                } else { ui.text_edit_singleline(model); }
                ui.label("Question to investigate");
                egui::ComboBox::from_id_salt("cycle_question").selected_text(self.question.and_then(|id|id.resolve(session).ok()).map_or("Choose question", |r|r.title.as_str())).width(540.0).show_ui(ui, |ui| {
                    for r in &session.records {
                        if let Ok(id) = QuestionId::new(r.id.0) && id.resolve(session).is_ok() {
                            ui.selectable_value(&mut self.question, Some(id), format!("#{} / {}", r.id.0, r.title));
                        }
                    }
                });
                ui.label("Objective / what this cycle should establish");
                ui.add(egui::TextEdit::multiline(&mut self.objective).desired_rows(2).desired_width(f32::INFINITY).char_limit(4000));
                ui.horizontal_wrapped(|ui| {
                    ui.label("Image offset");
                    ui.add(egui::TextEdit::singleline(&mut self.offset).desired_width(90.0));
                    ui.label("Bytes allowed"); ui.add(egui::DragValue::new(&mut self.length).range(1..=65536));
                    ui.label("Probe budget"); ui.add(egui::DragValue::new(&mut self.budget).range(1..=32));
                });
                match target {
                    Ok(t) => { ui.label(RichText::new(format!("{} / {} source bytes / sha256 {}", t.operation().label(), t.length(), &t.identity()[..16])).monospace().small()); }
                    Err(e) => { ui.label(e); }
                }
                let offset = self.offset.trim().strip_prefix("0x").map_or_else(||self.offset.trim().parse::<u64>(), |s|u64::from_str_radix(s,16));
                let valid = offset.as_ref().is_ok_and(|o|target.as_ref().is_ok_and(|t| o.checked_add(self.length).is_some_and(|end|end<=t.length()))) && self.question.is_some() && !model.trim().is_empty() && !self.objective.trim().is_empty();
                ui.label("Start authorizes sequential probes within this span. Pause finishes current work and holds the next step. Stop cancels current work and prevents follow-ups.");
                if ui.add_enabled(valid, egui::Button::new("Start bounded cycle")).clicked() {
                    action = Action::Start(Settings { question:self.question.unwrap(), model:model.clone(), budget:self.budget, offset:offset.unwrap(), length:self.length, objective:self.objective.clone() });
                }
            });
            ui.label(RichText::new("Identical completed probes reuse their original evidence. Draft text is provisional. Saved cases never resume execution automatically.").small().color(MUTED));
            ui.collapsing("Provisional OMP text", |ui| { ui.add(egui::Label::new(RichText::new(&engine.draft).monospace()).wrap()); });
            ui.collapsing("Evidence context for the next model request", |ui| {
                if let Ok(snapshot) = engine.snapshot(session) { ui.add(egui::Label::new(RichText::new(serde_json::to_string_pretty(&snapshot).unwrap()).monospace().small()).wrap()); }
                else { ui.label("Context becomes available when a cycle starts."); }
            });
        });
        self.open = open;
        action
    }
}
