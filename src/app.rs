use crate::graph::{self, ACCENT, BACKGROUND, GraphView, Layout, MUTED, PANEL, TEXT};
use divulge::{
    domain::{Artifact, Attention, Investigation, Origin, RecordData},
    storage,
};
use eframe::egui::{self, Color32, FontId, RichText, Stroke};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

enum FileCommand {
    Workshop,
    OpenArtifact,
    OpenSession,
    Save,
    SaveAs,
}

pub struct DivulgeApp {
    session: Investigation,
    attention: Attention,
    graph: GraphView,
    inspector: crate::insight::Inspector,
    playing: bool,
    next_tick: Instant,
    show_tree: bool,
    show_insight: bool,
    show_types: bool,
    show_about: bool,
    show_help: bool,
    show_activity: bool,
    presenter: bool,
    filter: String,
    status: String,
    error: Option<String>,
    activity: Vec<String>,
    save_path: Option<PathBuf>,
    dirty: bool,
    pending: Option<FileCommand>,
    confirm_discard: bool,
    pending_close: bool,
    artifact_worker: Option<mpsc::Receiver<Result<Artifact, String>>>,
    imported_fresh: bool,
}

impl DivulgeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);
        let session = Investigation::workshop();
        let mut attention = Attention::all(session.records.len());
        attention.select(3);
        let mut graph = GraphView::default();
        graph.reset(&session);
        let mut app = Self {
            session,
            attention,
            graph,
            inspector: crate::insight::Inspector::default(),
            playing: false,
            next_tick: Instant::now(),
            show_tree: true,
            show_insight: true,
            show_types: true,
            show_about: false,
            show_help: false,
            show_activity: false,
            presenter: false,
            filter: String::new(),
            status: "Workshop fixture ready. Select a node to inspect its evidence.".into(),
            error: None,
            activity: vec![
                "Loaded synthetic teaching fixture. No model or analysis tool was invoked.".into(),
            ],
            save_path: None,
            dirty: false,
            pending: None,
            confirm_discard: false,
            pending_close: false,
            artifact_worker: None,
            imported_fresh: false,
        };
        let args: Vec<String> = std::env::args().collect();
        if let Some(index) = args.iter().position(|arg| arg == "--artifact")
            && let Some(path) = args.get(index + 1)
        {
            app.inspect(PathBuf::from(path), &cc.egui_ctx);
        }
        app
    }

    fn selected(&self) -> &divulge::domain::Record {
        &self.session.records[self.attention.selected_index]
    }

    fn request(&mut self, command: FileCommand) {
        if self.artifact_worker.is_some() {
            self.status = "Wait for the current file inspection to finish.".into();
            return;
        }
        let replaces = matches!(
            command,
            FileCommand::Workshop | FileCommand::OpenArtifact | FileCommand::OpenSession
        );
        self.pending = Some(command);
        self.confirm_discard = replaces && self.dirty;
    }

    fn replace(&mut self, session: Investigation, fresh: bool) {
        self.session = session;
        self.attention = Attention::all(self.session.records.len());
        self.attention.select(3.min(self.session.records.len() - 1));
        self.graph.reset(&self.session);
        self.playing = false;
        self.save_path = None;
        self.dirty = false;
        self.imported_fresh = fresh;
        self.filter.clear();
    }

    fn inspect(&mut self, path: PathBuf, ctx: &egui::Context) {
        let (send, receive) = mpsc::channel();
        let ctx = ctx.clone();
        self.playing = false;
        self.artifact_worker = Some(receive);
        self.status = "Reading the selected file and calculating SHA-256...".into();
        std::thread::spawn(move || {
            let _ = send.send(storage::inspect_artifact(&path));
            ctx.request_repaint();
        });
    }

    fn process_pending(&mut self, ctx: &egui::Context) {
        if self.confirm_discard {
            return;
        }
        let Some(command) = self.pending.take() else {
            return;
        };
        match command {
            FileCommand::Workshop => {
                self.replace(Investigation::workshop(), false);
                self.status = "Loaded the synthetic workshop fixture.".into();
            }
            FileCommand::OpenArtifact => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Open an artifact for read-only inspection")
                    .pick_file()
                {
                    self.inspect(path, ctx);
                }
            }
            FileCommand::OpenSession => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Open a DIVULGE investigation")
                    .add_filter("DIVULGE investigation", &["json"])
                    .pick_file()
                {
                    match storage::load(&path) {
                        Ok(session) => {
                            self.replace(session, false);
                            self.save_path = Some(path);
                            self.status = "Opened recorded investigation. Replay does not execute tools or contact a model.".into();
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
            }
            FileCommand::Save | FileCommand::SaveAs => {
                let path = if matches!(command, FileCommand::Save) {
                    self.save_path.clone()
                } else {
                    None
                };
                let path = path.or_else(|| {
                    rfd::FileDialog::new()
                        .set_title("Save investigation")
                        .set_file_name("investigation.divulge.json")
                        .add_filter("DIVULGE investigation", &["json"])
                        .save_file()
                });
                if let Some(path) = path {
                    match storage::save(&path, &self.session) {
                        Ok(()) => {
                            self.save_path = Some(path);
                            self.dirty = false;
                            self.status =
                                "Investigation saved with records, notes, and evidence links."
                                    .into();
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
            }
        }
    }

    fn restart_replay(&mut self) {
        self.attention = Attention {
            visible_count: 1,
            selected_index: 0,
            following: true,
        };
        self.next_tick = Instant::now() + Duration::from_millis(1600);
        self.playing = true;
        self.status = "Replaying saved records. No model or tool calls.".into();
    }

    fn menus(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New workshop investigation").clicked() { self.request(FileCommand::Workshop); ui.close(); }
                if ui.button("Open artifact...                 Ctrl+O").clicked() { self.request(FileCommand::OpenArtifact); ui.close(); }
                if ui.button("Open investigation...   Ctrl+Shift+O").clicked() { self.request(FileCommand::OpenSession); ui.close(); }
                ui.separator();
                if ui.button("Save investigation              Ctrl+S").clicked() { self.request(FileCommand::Save); ui.close(); }
                if ui.button("Save investigation as...").clicked() { self.request(FileCommand::SaveAs); ui.close(); }
                ui.separator();
                if ui.button("Exit").clicked() { ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close); }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Copy selected insight").clicked() {
                    let selected = self.selected();
                    ui.ctx().copy_text(format!("{}\n{}\n\nEvidence: {}\nUncertainty: {}", selected.title, selected.insight.explanation, selected.insight.observation, selected.insight.uncertainty));
                    ui.close();
                }
                if ui.button("Find in investigation          Ctrl+F").clicked() { self.show_tree = true; ui.memory_mut(|memory| memory.request_focus(egui::Id::new("record_filter"))); ui.close(); }
            });
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.show_tree, "Investigation tree");
                ui.checkbox(&mut self.show_insight, "Harness Insight");
                ui.checkbox(&mut self.show_types, "Rust type annotations");
                ui.checkbox(&mut self.show_activity, "Activity log");
                ui.separator();
                if ui.button("Fit graph                            F").clicked() { self.graph.fit(); ui.close(); }
                if ui.checkbox(&mut self.presenter, "Presenter text size").changed() { ui.ctx().set_zoom_factor(if self.presenter { 1.15 } else { 1.0 }); }
            });
            ui.menu_button("Investigation", |ui| {
                if ui.button("Replay from beginning").clicked() { self.restart_replay(); ui.close(); }
                if ui.button("Show all records").clicked() { self.playing = false; self.attention = Attention::all(self.session.records.len()); ui.close(); }
                if ui.button("Follow latest record").clicked() { self.attention.follow(); ui.close(); }
                if ui.button("Review next record").clicked() { self.attention.select((self.attention.selected_index + 1).min(self.attention.visible_count - 1)); ui.close(); }
            });
            ui.menu_button("Analysis", |ui| {
                if ui.button("Identify local artifact...").clicked() { self.request(FileCommand::OpenArtifact); ui.close(); }
                ui.add_enabled(false, egui::Button::new("Decode FWSC package")).on_hover_text("Planned: a checked parser and exact V15 identity gate.");
                ui.add_enabled(false, egui::Button::new("Disassemble pi32v2")).on_hover_text("Planned: an explicit vendor tool adapter with validated instruction semantics.");
                ui.add_enabled(false, egui::Button::new("Run AI investigation")).on_hover_text("Planned: the restricted OMP bridge and bounded tool executor.");
            });
            ui.menu_button("Model", |ui| {
                ui.label(RichText::new("OMP is not connected in this build").color(MUTED));
                ui.label("The model catalogue will come from your OMP installation.");
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Workshop guide").clicked() { self.show_help = true; ui.close(); }
                if ui.button("About DIVULGE").clicked() { self.show_about = true; ui.close(); }
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("D / ").size(23.0).color(ACCENT));
            ui.label(RichText::new("DIVULGE").size(19.0).strong().color(TEXT));
            ui.add_space(10.0);
            ui.label(RichText::new("Investigation workbench").color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Save").clicked() {
                    self.request(FileCommand::Save);
                }
                if ui
                    .add_enabled(
                        self.artifact_worker.is_none(),
                        egui::Button::new("Open artifact"),
                    )
                    .clicked()
                {
                    self.request(FileCommand::OpenArtifact);
                }
                ui.separator();
                if ui
                    .button(if self.playing {
                        "Pause replay"
                    } else {
                        "Play replay"
                    })
                    .clicked()
                {
                    if self.playing {
                        self.playing = false;
                    } else if self.attention.visible_count == self.session.records.len() {
                        self.restart_replay();
                    } else {
                        self.playing = true;
                        self.next_tick = Instant::now() + Duration::from_millis(1600);
                    }
                }
                if ui
                    .add_enabled(
                        !self.playing && self.attention.visible_count < self.session.records.len(),
                        egui::Button::new("Step"),
                    )
                    .clicked()
                {
                    self.attention.advance(self.session.records.len());
                }
            });
        });
    }

    fn tree(&mut self, ui: &mut egui::Ui) {
        eyebrow(ui, "WORKSPACE");
        ui.add_space(8.0);
        ui.label(RichText::new(&self.session.title).strong().size(16.0));
        ui.add_space(4.0);
        ui.label(
            RichText::new(match self.session.origin {
                Origin::WorkshopFixture => "Synthetic teaching fixture",
                Origin::LocalArtifact => "Local artifact investigation",
            })
            .small()
            .color(MUTED),
        );
        ui.add_space(14.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .id(egui::Id::new("record_filter"))
                .hint_text("Find a record...")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(14.0);
        eyebrow(ui, "INVESTIGATION");
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt("tree_scroll")
            .show(ui, |ui| {
                let filter = self.filter.to_lowercase();
                for (i, record) in self
                    .session
                    .records
                    .iter()
                    .take(self.attention.visible_count)
                    .enumerate()
                {
                    if !format!("{} {}", record.title, record.data.label())
                        .to_lowercase()
                        .contains(&filter)
                    {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{:02}", record.id.0))
                                .monospace()
                                .small()
                                .color(graph::kind_color(&record.data)),
                        );
                        if ui
                            .selectable_label(i == self.attention.selected_index, &record.title)
                            .clicked()
                        {
                            self.attention.select(i);
                        }
                    });
                    ui.add_space(7.0);
                }
                ui.add_space(18.0);
                ui.separator();
                ui.add_space(10.0);
                eyebrow(ui, "ARTIFACT");
                ui.add_space(8.0);
                if let Some(artifact) = &self.session.artifact {
                    ui.label(&artifact.name);
                    ui.label(
                        RichText::new(format!("{} bytes", artifact.length))
                            .monospace()
                            .small()
                            .color(MUTED),
                    );
                    if artifact.matches_v15_reference() {
                        ui.label(
                            RichText::new(if self.imported_fresh {
                                "V15 reference matched"
                            } else {
                                "Recorded V15 hash match"
                            })
                            .color(ACCENT),
                        );
                    }
                    ui.collapsing("SHA-256", |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(&artifact.sha256).monospace().size(10.0),
                            )
                            .wrap(),
                        );
                    });
                } else {
                    ui.label(RichText::new("[0x34, 0x12]").monospace().color(ACCENT));
                    ui.label(
                        RichText::new("Two teaching bytes. No firmware is bundled.")
                            .small()
                            .color(MUTED),
                    );
                }
                ui.add_space(22.0);
                eyebrow(ui, "SESSION");
                ui.label(format!(
                    "{} records / {} open unknowns",
                    self.session.records.len(),
                    self.session
                        .records
                        .iter()
                        .filter(|r| matches!(r.data, RecordData::Unknown { .. }))
                        .count()
                ));
                ui.label(RichText::new("Model: not connected").small().color(MUTED));
                ui.add_space(12.0);
                if ui.button("Workshop guide").clicked() {
                    self.show_help = true;
                }
            });
    }

    fn insight(&mut self, ui: &mut egui::Ui) {
        let response = self.inspector.show(
            ui,
            &mut self.session,
            self.attention.selected_index,
            self.attention.visible_count,
            self.show_types,
        );
        if let Some(index) = response.selected_record {
            self.attention.select(index);
        }
        self.dirty |= response.note_changed;
    }

    fn focus_question(&mut self) {
        if let Some(question) = self.session.question_for(self.selected().id)
            && let Some(index) = self
                .session
                .records
                .iter()
                .position(|r| r.id == question.id)
            && index < self.attention.visible_count
        {
            self.attention.select(index);
            self.show_insight = true;
            self.graph
                .focus_question(&self.session, index, self.attention.visible_count);
        }
    }

    fn center(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Investigation graph").size(19.0).strong());
                ui.label(
                    RichText::new("Select to understand. Drag to arrange. Scroll to explore.")
                        .small()
                        .color(MUTED),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Fit all").clicked() {
                    self.graph.fit();
                }
                if ui.add_enabled(self.session.question_for(self.selected().id).is_some(), egui::Button::new("Focus question"))
                    .on_hover_text("Zoom to the question and its next probe; open why, how, and what the answer enables.")
                    .clicked()
                { self.focus_question(); }
            });
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let previous = self.graph.layout;
            ui.selectable_value(&mut self.graph.layout, Layout::Narrative, "Narrative");
            ui.selectable_value(
                &mut self.graph.layout,
                Layout::Relationships,
                "Relationships",
            );
            if previous != self.graph.layout {
                self.graph.reset(&self.session);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(if self.session.origin == Origin::WorkshopFixture {
                        "WORKSHOP FIXTURE"
                    } else {
                        "RECORDED OBSERVATIONS"
                    })
                    .size(10.0)
                    .color(MUTED),
                );
            });
        });
        ui.add_space(8.0);
        let selected_id = self.selected().id;
        let response =
            self.graph
                .show(ui, &self.session, self.attention.visible_count, selected_id);
        if let Some(index) = response.selected {
            self.attention.select(index);
        }
        if response.moved {
            self.attention.following = false;
        }
        if response.focus_question {
            self.focus_question();
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        egui::Window::new("About DIVULGE").open(&mut self.show_about).resizable(false).show(ctx, |ui| {
            ui.heading("DIVULGE 0.1.0");
            ui.label("An investigation workbench for understanding how we know.");
            ui.label("Native Rust + egui. MIT licensed project code.");
            ui.separator();
            ui.label("Foundation build: artifact identity, evidence graph, Harness Insight, notes, save/open, and replay.");
            ui.label("REA, pi32v2 decoding, and the OMP AI executor are planned integrations.");
            ui.label("Repository: github.com/superposition/DIVULGE");
        });
        egui::Window::new("Workshop / From bytes to evidence").open(&mut self.show_help).default_width(530.0).show(ctx, |ui| {
            ui.heading("Learn the types. Follow the evidence.");
            ui.add_space(8.0);
            for line in [
                "1. Select 'Two bytes: 34 12' and inspect the observed bytes.",
                "2. Select 'One interpretation: 4660'. Follow its evidence link.",
                "3. Explain why the meaning of 4660 is still unknown.",
                "4. Play replay, then select an earlier record. Notice that your reading position stays put.",
                "5. Use Review next or Follow latest to catch up.",
                "6. Add a note and save the investigation. Reopen it to replay without tools.",
            ] { ui.label(line); ui.add_space(6.0); }
            ui.separator();
            ui.monospace("cargo run --example workshop_types");
            ui.label("The PDF workshop is tracked in the repository roadmap. This example uses synthetic bytes; it makes no claim about FM-1 behavior.");
        });
        if let Some(error) = self.error.clone() {
            egui::Window::new("Operation could not complete")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(error);
                    if ui.button("Close").clicked() {
                        self.error = None;
                    }
                });
        }
        if self.confirm_discard || self.pending_close {
            egui::Window::new("Unsaved notes")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Your current investigation has unsaved notes.");
                    ui.horizontal(|ui| {
                        if ui.button("Keep editing").clicked() {
                            self.pending = None;
                            self.confirm_discard = false;
                            self.pending_close = false;
                        }
                        if ui.button("Discard and continue").clicked() {
                            self.dirty = false;
                            self.confirm_discard = false;
                            if self.pending_close {
                                self.pending_close = false;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                    });
                });
        }
    }
}

impl eframe::App for DivulgeApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|input| input.viewport().close_requested()) && self.dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_close = true;
        }
        if let Some(receiver) = &self.artifact_worker {
            match receiver.try_recv() {
                Ok(result) => {
                    self.artifact_worker = None;
                    match result {
                        Ok(artifact) => {
                            self.replace(Investigation::from_artifact(artifact), true);
                            self.dirty = true;
                            self.status = "Artifact identity recorded. Select its observation to inspect the bytes.".into();
                            self.activity.push(
                                "Completed local file hashing and bounded byte preview.".into(),
                            );
                        }
                        Err(error) => {
                            self.error = Some(error);
                            self.status =
                                "Artifact inspection failed; current investigation retained."
                                    .into();
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.artifact_worker = None;
                    self.error = Some("File inspection worker stopped unexpectedly".into());
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100))
                }
            }
        }
        if self.playing {
            if Instant::now() >= self.next_tick {
                self.attention.advance(self.session.records.len());
                self.next_tick = Instant::now() + Duration::from_millis(1600);
                if self.attention.visible_count == self.session.records.len() {
                    self.playing = false;
                    self.status = "Replay complete. Review at your own pace.".into();
                }
            }
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input_mut(|input| {
            input.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::O)
        }) {
            self.request(FileCommand::OpenSession);
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
            self.request(FileCommand::OpenArtifact);
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::S)) {
            self.request(FileCommand::Save);
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            self.show_tree = true;
            ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("record_filter")));
        }
        if !ctx.egui_wants_keyboard_input() && ctx.input(|input| input.key_pressed(egui::Key::F)) {
            self.graph.fit();
        }

        egui::Panel::top("menu")
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(10, 3)),
            )
            .show(ui, |ui| self.menus(ui));
        egui::Panel::top("toolbar")
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14, 11)),
            )
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("attention")
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mode = if self.playing { "REPLAYING" } else { "REVIEW" };
                    ui.label(RichText::new(mode).size(10.0).color(ACCENT));
                    ui.separator();
                    ui.label(format!(
                        "Reading {:02} / Latest {:02}",
                        self.selected().id.0,
                        self.session.records[self.attention.visible_count - 1].id.0
                    ));
                    if self.attention.unread() > 0 {
                        ui.label(
                            RichText::new(format!("{} ahead", self.attention.unread()))
                                .color(ACCENT),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .selectable_label(self.attention.following, "Follow latest")
                            .clicked()
                        {
                            self.attention.follow();
                        }
                        if ui
                            .add_enabled(
                                self.attention.unread() > 0,
                                egui::Button::new("Review next"),
                            )
                            .clicked()
                        {
                            self.attention.select(self.attention.selected_index + 1);
                        }
                        if self.dirty {
                            ui.label(RichText::new("Unsaved").small().color(MUTED));
                        }
                    });
                });
            });
        if self.show_activity {
            egui::Panel::bottom("activity")
                .default_size(110.0)
                .show(ui, |ui| {
                    ui.label(&self.status);
                    for entry in self.activity.iter().rev().take(3) {
                        ui.label(RichText::new(entry).monospace().small().color(MUTED));
                    }
                });
        }
        if self.show_tree {
            egui::Panel::left("tree")
                .default_size(205.0)
                .size_range(160.0..=320.0)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(14))
                .show(ui, |ui| self.tree(ui));
        }
        if self.show_insight {
            egui::Panel::right("insight")
                .default_size(420.0)
                .size_range(360.0..=680.0)
                .frame(
                    egui::Frame::new()
                        .fill(crate::insight::SURFACE)
                        .inner_margin(14),
                )
                .show(ui, |ui| self.insight(ui));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BACKGROUND).inner_margin(18))
            .show(ui, |ui| self.center(ui));
        self.dialogs(&ctx);
        self.process_pending(&ctx);
    }
}

fn eyebrow(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(10.0).color(MUTED));
}

fn configure_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(11.0));
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(9.0, 5.0);
    style.visuals.panel_fill = PANEL;
    style.visuals.window_fill = PANEL;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.selection.bg_fill = Color32::from_rgb(42, 72, 71);
    style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    style.visuals.hyperlink_color = ACCENT;
    style.visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(1.0, Color32::from_rgb(49, 59, 70));
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(37, 46, 57);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(50, 64, 74);
    ctx.set_style_of(egui::Theme::Dark, style);
}
