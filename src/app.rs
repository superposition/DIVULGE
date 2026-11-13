use crate::graph::{self, ACCENT, BACKGROUND, GraphView, MUTED, PANEL, TEXT};
use divulge::{
    coordinates::ImageOffset,
    decoded_image::DecodedImage,
    domain::{Attention, Investigation, Origin, RecordData, RecordId},
    pi32::{self, DisassemblyReport, RunId},
    storage,
};
use eframe::egui::{self, Color32, FontId, RichText, Stroke};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

enum FileCommand {
    Workshop,
    OpenArtifact,
    OpenSession,
    RecoverSession,
    Save,
    SaveAs,
    VerifySource,
}

struct CodeJob {
    history_id: String,
    run_id: RunId,
    receiver: mpsc::Receiver<Result<DisassemblyReport, String>>,
    cancel: Arc<AtomicBool>,
    generation: u64,
    reading: (RecordId, Option<RunId>, usize),
}

pub struct DivulgeApp {
    session: Investigation,
    history: divulge::history::History,
    history_view: crate::history_view::HistoryView,
    history_dirty: bool,
    tracked_jobs: std::collections::BTreeMap<String, (String, Option<RunId>)>,
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
    show_data: bool,
    show_code: bool,
    code: crate::code_view::CodeView,
    image: Option<Arc<DecodedImage>>,
    code_job: Option<CodeJob>,
    generation: u64,
    attach_source: bool,
    close_after_job: bool,
    ai: crate::ai_view::AiView,
    engine: divulge::engine::Engine,
    engine_view: crate::engine_view::EngineView,
    rea: crate::rea_view::ReaView,
    source_path: Option<PathBuf>,
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
    artifact_worker: Option<mpsc::Receiver<Result<storage::Inspection, String>>>,
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
            history: divulge::history::History::new(
                &session,
                "Synthetic teaching baseline; no tools executed",
            )
            .unwrap(),
            history_view: Default::default(),
            history_dirty: false,
            tracked_jobs: Default::default(),
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
            show_data: false,
            show_code: false,
            code: crate::code_view::CodeView::default(),
            image: None,
            code_job: None,
            generation: 0,
            attach_source: false,
            close_after_job: false,
            ai: crate::ai_view::AiView::default(),
            engine: Default::default(),
            engine_view: Default::default(),
            rea: crate::rea_view::ReaView::default(),
            source_path: None,
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
        let mut restored = false;
        if let Some(index) = args.iter().position(|arg| arg == "--session")
            && let Some(path) = args.get(index + 1)
        {
            match storage::load_history(&PathBuf::from(path)) {
                Ok(history) => {
                    app.replace(history.session().clone(), false);
                    app.history = history;
                    app.save_path = Some(PathBuf::from(path));
                    restored = true;
                }
                Err(error) => app.error = Some(error),
            }
        }
        if let Some(index) = args.iter().position(|arg| arg == "--artifact")
            && let Some(path) = args.get(index + 1)
        {
            app.inspect(PathBuf::from(path), &cc.egui_ctx, restored);
        }
        app
    }

    fn selected(&self) -> &divulge::domain::Record {
        &self.session.records[self.attention.selected_index]
    }

    fn capture_history(&mut self) {
        if self.history_dirty {
            if let Err(error) = self.history.capture(&self.session) {
                self.error = Some(error);
            }
            self.history_dirty = false;
        }
    }
    fn sync_jobs(&mut self) {
        let mut active = std::collections::BTreeMap::new();
        if let Some(job) = &self.code_job {
            active.insert(
                job.history_id.clone(),
                ("pi32 disassembly".into(), Some(job.run_id)),
            );
        }
        if let Some(job) = &self.ai.job
            && job.propose
        {
            active.insert(job.id.clone(), ("OMP proposal".into(), None));
        }
        if let Some(job) = &self.rea.job
            && !matches!(job.request, divulge::rea::Request::Discover)
        {
            active.insert(job.id.clone(), ("REA evidence".into(), None));
        }
        if active == self.tracked_jobs {
            return;
        }
        self.capture_history();
        for (id, (_, run_id)) in &self.tracked_jobs {
            if !active.contains_key(id) {
                let outcome = self
                    .session
                    .ai_runs
                    .iter()
                    .find(|r| r.id == *id)
                    .map(|r| r.outcome.clone())
                    .or_else(|| {
                        self.session
                            .rea_runs
                            .iter()
                            .find(|r| r.receipt.id == *id)
                            .map(|r| r.receipt.kind.clone())
                    })
                    .or_else(|| {
                        run_id
                            .and_then(|n| self.session.disassemblies.iter().find(|r| r.id == n))
                            .map(|r| format!("{:?}", r.capture.outcome))
                    })
                    .unwrap_or_else(|| {
                        "Ended without an accepted receipt; no result claimed".into()
                    });
                if let Err(e) = self.history.finish_job(id, &outcome) {
                    self.error = Some(e);
                }
            }
        }
        for (id, (adapter, _)) in &active {
            if !self.tracked_jobs.contains_key(id)
                && let Err(e) = self.history.start_job(id, adapter)
            {
                self.error = Some(e);
            }
        }
        self.tracked_jobs = active;
        self.dirty = true;
    }

    fn request(&mut self, command: FileCommand) {
        if !matches!(command, FileCommand::Save | FileCommand::SaveAs)
            && (self.artifact_worker.is_some()
                || self.code_job.is_some()
                || self.ai.job.is_some()
                || self.rea.job.is_some()
                || self.engine.busy())
        {
            self.status = "Finish or cancel the active job before changing files.".into();
            return;
        }
        let replaces = matches!(
            command,
            FileCommand::Workshop
                | FileCommand::OpenArtifact
                | FileCommand::OpenSession
                | FileCommand::RecoverSession
        );
        self.pending = Some(command);
        self.confirm_discard = replaces && self.dirty;
    }

    fn replace(&mut self, session: Investigation, fresh: bool) {
        self.history = divulge::history::History::new(
            &session,
            "Starting investigation snapshot; prior chronology unavailable",
        )
        .expect("validated replacement");
        self.history_view = Default::default();
        self.history_dirty = false;
        self.tracked_jobs.clear();
        self.ai.reset();
        self.engine.invalidate();
        self.engine_view = Default::default();
        self.rea.reset();
        self.source_path = None;
        self.generation = self.generation.wrapping_add(1);
        self.image = None;
        let config = self.code.config.clone();
        self.code = crate::code_view::CodeView::default();
        self.code.config = config;
        self.show_code = false;
        self.session = session;
        self.attention = Attention::all(self.session.records.len());
        self.attention.select(3.min(self.session.records.len() - 1));
        self.show_data = self
            .session
            .artifact
            .as_ref()
            .is_some_and(|a| a.firmware.is_some());
        if self.show_data
            && let Some(index) = self.session.records.iter().position(|r| {
                matches!(
                    r.data,
                    RecordData::Firmware(divulge::fwsc::FirmwareView::Application)
                )
            })
        {
            self.attention.select(index);
        }
        self.graph.reset(&self.session);
        if let Some(report) = self.session.disassemblies.last() {
            self.code.select_run(report);
            self.show_code = true;
            if let Some(index) = self
                .session
                .records
                .iter()
                .position(|r| matches!(r.data, RecordData::Disassembly(id) if id == report.id))
            {
                self.attention.select(index);
            }
        }
        self.playing = false;
        self.save_path = None;
        self.dirty = false;
        self.imported_fresh = fresh;
        self.filter.clear();
    }

    fn inspect(&mut self, path: PathBuf, ctx: &egui::Context, attach: bool) {
        let (send, receive) = mpsc::channel();
        let ctx = ctx.clone();
        self.playing = false;
        self.artifact_worker = Some(receive);
        self.attach_source = attach;
        self.status =
            "Reading and hashing the file; decoding if the exact V15 identity matches...".into();
        std::thread::spawn(move || {
            let _ = send.send(storage::inspect_with_image(&path));
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
                    self.inspect(path, ctx, false);
                }
            }
            FileCommand::OpenSession | FileCommand::RecoverSession => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Open a DIVULGE investigation")
                    .add_filter("DIVULGE investigation", &["json"])
                    .pick_file()
                {
                    let recovery = matches!(command, FileCommand::RecoverSession);
                    let source = if recovery {
                        storage::backup_path(&path)
                    } else {
                        path.clone()
                    };
                    match storage::load_history(&source) {
                        Ok(history) => {
                            self.replace(history.session().clone(), false);
                            self.history = history;
                            self.save_path = if recovery { None } else { Some(path) };
                            self.status = "Opened recorded investigation. Replay does not execute tools or contact a model.".into();
                            if recovery {
                                self.status="Recovered the previous save for inspection. Save As keeps the damaged primary available for diagnosis.".into();
                            }
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
            }
            FileCommand::VerifySource => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Verify the original artifact for this investigation")
                    .pick_file()
                {
                    self.inspect(path, ctx, true);
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
                    let saved = self
                        .history
                        .capture(&self.session)
                        .and_then(|_| storage::save_history(&path, &self.history));
                    match saved {
                        Ok(()) => {
                            self.save_path = Some(path);
                            self.dirty = false;
                            self.history_dirty = false;
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
        self.attention = Attention::all(1);
        self.next_tick = Instant::now() + Duration::from_millis(1600);
        self.playing = true;
        self.status = "Replaying saved records. No model or tool calls.".into();
    }

    fn open_code(&mut self) {
        self.show_code = true;
        let report = self.session.disassemblies.iter().rev().find(|report| {
            self.session
                .records
                .iter()
                .take(self.attention.visible_count)
                .any(|r| matches!(r.data, RecordData::Disassembly(id) if id == report.id))
        });
        if let Some(report) = report {
            self.code.select_run(report);
            if let Some(index) = self
                .session
                .records
                .iter()
                .position(|r| matches!(r.data, RecordData::Disassembly(id) if id == report.id))
            {
                self.attention.select(index);
            }
        }
    }

    fn start_disassembly(&mut self, offset: u64, length: u64, ctx: &egui::Context) {
        if self.engine.busy()
            || self.code_job.is_some()
            || self.artifact_worker.is_some()
            || self.ai.job.is_some()
            || self.rea.job.is_some()
        {
            return;
        }
        let Some(image) = self.image.clone() else {
            self.error =
                Some("Verify the original artifact before running a new disassembly.".into());
            return;
        };
        if self.session.disassemblies.len() >= 16 || self.session.records.len() + 3 > 200 {
            self.error =
                Some("This investigation has reached its bounded run/record limit.".into());
            return;
        }
        let Some(id) = self
            .session
            .disassemblies
            .iter()
            .map(|r| r.id.0)
            .max()
            .unwrap_or(0)
            .checked_add(1)
        else {
            self.error = Some("Run ID overflow".into());
            return;
        };
        let config = self.code.config.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let (send, receiver) = mpsc::channel();
        let context = ctx.clone();
        self.playing = false;
        self.show_code = true;
        self.code_job = Some(CodeJob {
            history_id: format!(
                "pi32-{}-{id}-{}",
                self.generation,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ),
            run_id: RunId(id),
            receiver,
            cancel,
            generation: self.generation,
            reading: (
                self.selected().id,
                self.code.selected_run,
                self.code.selected_instruction,
            ),
        });
        self.status = format!(
            "Disassembling app.bin+0x{offset:X}, {length} bytes. You can keep reading or cancel."
        );
        std::thread::spawn(move || {
            let result = pi32::disassemble(
                &image,
                RunId(id),
                ImageOffset(offset),
                length,
                &config,
                &worker_cancel,
            );
            let _ = send.send(result);
            context.request_repaint();
        });
    }

    fn code_action(&mut self, action: crate::code_view::Action, ctx: &egui::Context) {
        if action.cancel
            && let Some(job) = &self.code_job
        {
            job.cancel.store(true, Ordering::Relaxed);
            self.status = "Cancelling the owned tool process; retaining available output...".into();
        }
        if action.verify_source {
            self.request(FileCommand::VerifySource);
        }
        if let Some(id) = action.selected_run
            && let Some(index) = self
                .session
                .records
                .iter()
                .position(|r| matches!(r.data, RecordData::Disassembly(run) if run == id))
        {
            self.attention.select(index);
        }
        if let Some((offset, length)) = action.request {
            self.start_disassembly(offset, length, ctx);
        }
    }

    fn menus(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New workshop investigation").clicked() {
                    self.request(FileCommand::Workshop);
                    ui.close();
                }
                if ui
                    .button("Open artifact...                 Ctrl+O")
                    .clicked()
                {
                    self.request(FileCommand::OpenArtifact);
                    ui.close();
                }
                if ui.button("Open investigation...   Ctrl+Shift+O").clicked() {
                    self.request(FileCommand::OpenSession);
                    ui.close();
                }
                if ui.button("Recover previous save...").clicked() {
                    self.request(FileCommand::RecoverSession);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.session.artifact.is_some(),
                        egui::Button::new("Verify source for this investigation..."),
                    )
                    .clicked()
                {
                    self.request(FileCommand::VerifySource);
                    ui.close();
                }
                ui.separator();
                if ui
                    .button("Save investigation              Ctrl+S")
                    .clicked()
                {
                    self.request(FileCommand::Save);
                    ui.close();
                }
                if ui.button("Save investigation as...").clicked() {
                    self.request(FileCommand::SaveAs);
                    ui.close();
                }
                if ui.button("Export portable review...").clicked() {
                    self.history_view.export_open = true;
                    ui.close();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Copy selected insight").clicked() {
                    let selected = self.selected();
                    ui.ctx().copy_text(format!(
                        "{}\n{}\n\nEvidence: {}\nUncertainty: {}",
                        selected.title,
                        selected.insight.explanation,
                        selected.insight.observation,
                        selected.insight.uncertainty
                    ));
                    ui.close();
                }
                if ui.button("Find in investigation          Ctrl+F").clicked() {
                    self.show_tree = true;
                    ui.memory_mut(|memory| memory.request_focus(egui::Id::new("record_filter")));
                    ui.close();
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Case history and review...").clicked() {
                    self.history_view.open = true;
                    ui.close();
                }
                ui.checkbox(&mut self.show_tree, "Investigation tree");
                ui.checkbox(&mut self.show_insight, "Harness Insight");
                ui.checkbox(&mut self.show_types, "Rust type annotations");
                ui.checkbox(&mut self.show_activity, "Activity log");
                ui.menu_button("Panel widths", |ui| {
                    for (id, label, min, max, default) in [("tree", "Investigation tree", 160.0, 320.0, 205.0), ("insight", "Harness Insight", 360.0, 680.0, 420.0)] {
                        let key = egui::Id::new(id);
                        let state = egui::containers::panel::PanelState::load(ui.ctx(), key);
                        let mut width = state.map(|s| s.size().x).unwrap_or(default);
                        if ui.add(egui::Slider::new(&mut width, min..=max).text(label)).changed()
                            && let Some(mut state) = state {
                            if id == "tree" { state.outer_rect.max.x = state.outer_rect.min.x + width; }
                            else { state.outer_rect.min.x = state.outer_rect.max.x - width; }
                            ui.ctx().data_mut(|data| data.insert_persisted(key, state));
                        }
                    }
                    ui.label("Use sliders or their value fields; arrow keys adjust a focused slider.");
                });
                ui.separator();
                if ui
                    .button("Fit graph                            F")
                    .clicked()
                {
                    self.graph.fit();
                    ui.close();
                }
                if ui
                    .checkbox(&mut self.presenter, "Presenter text size")
                    .changed()
                {
                    ui.ctx()
                        .set_zoom_factor(if self.presenter { 1.15 } else { 1.0 });
                }
            });
            ui.menu_button("Investigation", |ui| {
                if ui.button("Replay from beginning").clicked() {
                    self.restart_replay();
                    ui.close();
                }
                if ui.button("Show all records").clicked() {
                    self.playing = false;
                    self.attention = Attention::all(self.session.records.len());
                    ui.close();
                }
                if ui.button("Follow latest record").clicked() {
                    self.follow_latest();
                    ui.close();
                }
                if ui.button("Review next record").clicked() {
                    self.review_next();
                    ui.close();
                }
                ui.checkbox(&mut self.attention.held, "Presenter hold")
                    .on_hover_text("Keep the reading selection and text fixed while execution continues. Follow latest releases the hold.");
            });
            ui.menu_button("Analysis", |ui| {
                if ui.button("Identify local artifact...").clicked() {
                    self.request(FileCommand::OpenArtifact);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.session
                            .artifact
                            .as_ref()
                            .is_some_and(|a| a.firmware.is_some()),
                        egui::Button::new("Show decoded V15 data"),
                    )
                    .on_hover_text("The exact V15 package is decoded automatically on import.")
                    .clicked()
                {
                    self.show_data = true;
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.session.artifact.as_ref().is_some_and(|a| {
                            a.firmware.as_ref().is_some_and(|f| f.application.is_some())
                        }),
                        egui::Button::new("Disassemble pi32v2"),
                    )
                    .clicked()
                {
                    self.open_code();
                    ui.close();
                }
                if ui.button("REA original-file evidence...").clicked() {
                    self.rea.open = true;
                    ui.close();
                }
                if ui.button("Propose next step with OMP...").clicked() {
                    self.ai.open = true;
                    ui.close();
                }
            });
            ui.menu_button("Model", |ui| {
                if ui.button("Run bounded investigation...").clicked() {
                    self.engine_view.prepare(&self.session, self.selected().id);
                    ui.close();
                }
                if ui.button("OMP models and next step...").clicked() {
                    self.ai.open = true;
                    ui.close();
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Workshop guide").clicked() {
                    self.show_help = true;
                    ui.close();
                }
                if ui.button("About DIVULGE").clicked() {
                    self.show_about = true;
                    ui.close();
                }
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
                if ui.button("AI cycle").clicked() {
                    self.engine_view.prepare(&self.session, self.selected().id);
                }
                if ui
                    .add_enabled(
                        self.artifact_worker.is_none() && self.code_job.is_none(),
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
        let filter_response = ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .id(egui::Id::new("record_filter"))
                .hint_text("Find ID, text or kind...")
                .desired_width(f32::INFINITY),
        );
        let mut select_match = (filter_response.has_focus() || filter_response.lost_focus())
            && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if select_match
            || filter_response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape))
        {
            filter_response.surrender_focus();
        }
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
                    if !format!(
                        "#{:04} {} {} {}",
                        record.id.0,
                        record.title,
                        record.data.label(),
                        record.summary
                    )
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
                        let response =
                            ui.selectable_label(i == self.attention.selected_index, &record.title);
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Button,
                                ui.is_enabled(),
                                i == self.attention.selected_index,
                                format!(
                                    "Record {}. {}. {}",
                                    record.id.0,
                                    record.data.label(),
                                    record.title
                                ),
                            )
                        });
                        if response.clicked() || select_match {
                            select_match = false;
                            self.attention.select(i);
                            if !self.show_code && !self.show_data {
                                self.graph.focus_result(
                                    &self.session,
                                    i,
                                    self.attention.visible_count,
                                );
                            }
                            if let RecordData::Disassembly(id) = record.data {
                                if let Some(report) =
                                    self.session.disassemblies.iter().find(|r| r.id == id)
                                {
                                    self.code.select_run(report);
                                }
                                self.show_code = true;
                            } else {
                                self.show_code = false;
                            }
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
                if ui.button("Ask OMP...").clicked() {
                    self.ai.open = true;
                }
                ui.add_space(12.0);
                if ui.button("Workshop guide").clicked() {
                    self.show_help = true;
                }
            });
    }

    fn insight(&mut self, ui: &mut egui::Ui) {
        if ui.rect_contains_pointer(ui.max_rect())
            && ui.input(|i| i.pointer.any_pressed() || i.smooth_scroll_delta != egui::Vec2::ZERO)
        {
            self.attention.following = false;
        }
        if (self.show_code || matches!(self.selected().data, RecordData::Disassembly(_)))
            && let Some(id) = self
                .code
                .report(&self.session, self.attention.visible_count)
                .map(|r| r.id)
            && let Some(run_index) = self.session.disassemblies.iter().position(|r| r.id == id)
            && let Some(record_index) = self
                .session
                .records
                .iter()
                .position(|r| matches!(r.data, RecordData::Disassembly(run) if run == id))
        {
            let changed = self.code.inspector(
                ui,
                &self.session.disassemblies[run_index],
                &mut self.session.records[record_index].note,
                self.show_types,
            );
            self.dirty |= changed;
            self.history_dirty |= changed;
            return;
        }
        let response = self.inspector.show(
            ui,
            &mut self.session,
            self.attention.selected_index,
            self.attention.reading_count(),
            self.show_types,
        );
        if let Some(index) = response.selected_record {
            self.attention.select(index);
        }
        self.dirty |= response.note_changed;
        self.history_dirty |= response.note_changed;
        if let Some(index) = response.created_record {
            self.dirty = true;
            self.history_dirty = true;
            self.attention.visible_count = self.session.records.len();
            self.attention.select(index);
            self.graph.append_records(&self.session);
            self.graph
                .focus_result(&self.session, index, self.attention.visible_count);
            self.show_code = false;
            self.show_data = false;
        }
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
            if ui
                .selectable_label(!self.show_data && !self.show_code, "Graph")
                .clicked()
            {
                self.show_code = false;
                self.show_data = false;
                self.graph.focus_result(
                    &self.session,
                    self.attention.selected_index,
                    self.attention.visible_count,
                );
            }
            if ui
                .selectable_label(self.show_data && !self.show_code, "Data")
                .clicked()
            {
                self.show_data = true;
                self.show_code = false;
            }
            if ui.selectable_label(self.show_code, "Code").clicked() {
                self.open_code();
            }
        });
        ui.separator();
        ui.add_space(8.0);
        if self.show_code {
            let action = self.code.show(
                ui,
                &self.session,
                self.attention.visible_count,
                self.image.is_some() && self.artifact_worker.is_none(),
                self.code_job.is_some(),
            );
            self.code_action(action, ui.ctx());
            return;
        }
        if self.show_data {
            if let Some(index) = crate::data_view::table(
                ui,
                &self.session,
                self.attention.visible_count,
                self.attention.selected_index,
            ) {
                self.attention.select(index);
            }
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Investigation graph").size(17.0).strong());
            if ui
                .add_enabled(
                    self.session.question_for(self.selected().id).is_some(),
                    egui::Button::new("Focus question"),
                )
                .on_hover_text("Show this question's branch and inspect why, how and to what end.")
                .clicked()
            {
                self.focus_question();
            }
        });
        let selected_id = self.selected().id;
        let response =
            self.graph
                .show(ui, &self.session, self.attention.visible_count, selected_id);
        if let Some(index) = response.selected {
            self.attention.select(index);
            if let RecordData::Disassembly(id) = self.session.records[index].data
                && let Some(report) = self.session.disassemblies.iter().find(|r| r.id == id)
            {
                self.code.select_run(report);
            }
        }
        if response.moved {
            self.attention.following = false;
        }
        if response.focus_question {
            self.focus_question();
        }
    }

    fn engine_action(&mut self, action: crate::engine_view::Action, ctx: &egui::Context) {
        use crate::engine_view::Action;
        if matches!(action, Action::None) {
            return;
        }
        self.capture_history();
        let result = match action {
            Action::None => Ok(()),
            Action::Models => {
                self.ai.open = true;
                self.engine_view.open = false;
                Ok(())
            }
            Action::Start(settings) => {
                self.playing = false;
                divulge::engine::Target::from_session(&self.session, self.image.clone()).and_then(
                    |target| {
                        self.engine.start(
                            &mut self.session,
                            &mut self.history,
                            target,
                            self.code.config.clone(),
                            settings,
                            self.generation,
                        )
                    },
                )
            }
            Action::Pause => self.engine.pause(&mut self.history),
            Action::Resume => self.engine.resume(&mut self.history),
            Action::Stop => self.engine.stop(&mut self.history),
        };
        if let Err(error) = result {
            self.error = Some(error);
        }
        self.dirty = true;
        self.refresh_execution_graph();
        ctx.request_repaint();
    }
    fn refresh_execution_graph(&mut self) {
        self.graph.append_records(&self.session);
        if self.attention.publish(self.session.records.len()) {
            self.focus_attention();
        }
    }
    fn focus_attention(&mut self) {
        self.show_code = false;
        self.show_data = false;
        self.graph.focus_result(
            &self.session,
            self.attention.selected_index,
            self.attention.visible_count,
        );
    }
    fn follow_latest(&mut self) {
        self.attention.follow();
        self.focus_attention();
    }
    fn review_next(&mut self) {
        if self.attention.review_next() {
            self.focus_attention();
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        let selected = self.selected().id;
        if let Some(index) = self
            .history_view
            .show(ctx, &self.history, &mut self.session, selected)
        {
            self.dirty = true;
            self.history_dirty = true;
            self.attention.visible_count = self.session.records.len();
            self.attention.select(index);
            self.graph.append_records(&self.session);
            self.show_code = false;
            self.show_data = false;
            self.graph
                .focus_result(&self.session, index, self.attention.visible_count);
        }
        if let Some(index) = self.rea.show(
            ctx,
            &self.session,
            self.source_path.as_deref(),
            self.generation,
            self.engine.busy()
                || self.code_job.is_some()
                || self.artifact_worker.is_some()
                || self.ai.job.is_some(),
        ) {
            self.attention.select(index);
            self.show_code = false;
            self.show_data = false;
            self.graph
                .focus_result(&self.session, index, self.attention.visible_count);
        }
        let action = self.ai.show(
            ctx,
            &self.session,
            self.attention.visible_count,
            self.selected().id,
            self.generation,
            self.engine.busy()
                || self.code_job.is_some()
                || self.artifact_worker.is_some()
                || self.rea.job.is_some(),
        );
        if let Some(index) = action.review {
            self.attention.select(index);
            self.show_code = false;
            self.show_data = false;
            self.graph
                .focus_result(&self.session, index, self.attention.visible_count);
        }
        if let Some((offset, length)) = action.stage {
            self.open_code();
            self.code.stage(offset, length);
        }
        let target = divulge::engine::Target::from_session(&self.session, self.image.clone());
        let action = self.engine_view.show(
            ctx,
            &self.session,
            &self.engine,
            &target,
            &mut self.ai.model,
            self.ai.catalogue.as_ref(),
            self.code_job.is_some()
                || self.artifact_worker.is_some()
                || self.rea.job.is_some()
                || self.ai.job.is_some(),
        );
        self.engine_action(action, ctx);
        egui::Window::new("About DIVULGE").open(&mut self.show_about).resizable(false).show(ctx, |ui| {
            ui.heading("DIVULGE 0.1.0");
            ui.label("An investigation workbench for understanding how we know.");
            ui.label("Native Rust + egui. MIT licensed project code.");
            ui.separator();
            ui.label("Foundation build: artifact identity, evidence graph, Harness Insight, notes, save/open, and replay.");
            ui.label("V15 decoding, pi32v2 disassembly, REA entropy evidence and bounded OMP investigation cycles are available.");
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
        if self.engine.controller.is_some() {
            // Runtime capture occurs at meaningful request/result boundaries, not each keystroke.
            match self
                .engine
                .tick(&mut self.session, &mut self.history, self.generation)
            {
                Ok(true) => {
                    self.dirty = true;
                    self.refresh_execution_graph();
                    ctx.request_repaint();
                }
                Ok(false) => {}
                Err(error) => {
                    self.dirty = true;
                    self.error = Some(error);
                }
            }
            if self.engine.busy() {
                ctx.request_repaint_after(Duration::from_millis(50));
            }
        }
        if self.rea.poll(&mut self.session, self.generation) {
            self.dirty = true;
            self.history_dirty = true;
            self.refresh_execution_graph();
        }
        if self.rea.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        if self.ai.poll(&mut self.session, self.generation) {
            self.dirty = true;
            self.history_dirty = true;
            self.refresh_execution_graph();
        }
        if self.ai.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        if self.close_after_job
            && self.ai.job.is_none()
            && self.code_job.is_none()
            && self.rea.job.is_none()
            && !self.engine.has_worker()
        {
            self.close_after_job = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ctx.input(|input| input.viewport().close_requested()) {
            if self.engine.busy() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                if let Err(error) = self.engine.stop(&mut self.history) {
                    self.error = Some(error);
                }
                self.dirty = true;
                self.close_after_job = true;
                ctx.request_repaint();
            } else if let Some(job) = &self.rea.job {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                job.cancel.store(true, Ordering::Relaxed);
                self.close_after_job = true;
            } else if let Some(job) = &self.ai.job {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                job.cancel.store(true, Ordering::Relaxed);
                self.close_after_job = true;
            } else if let Some(job) = &self.code_job {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                job.cancel.store(true, Ordering::Relaxed);
                self.close_after_job = true;
                self.status = "Cancelling the active tool before closing...".into();
            } else if self.dirty {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.pending_close = true;
            }
        }
        if let Some(job) = &self.code_job {
            match job.receiver.try_recv() {
                Ok(result) => {
                    let job = self.code_job.take().unwrap();
                    if job.generation == self.generation {
                        match result {
                            Ok(report) => {
                                let focus = self.show_code
                                    && !self.attention.held
                                    && job.reading
                                        == (
                                            self.selected().id,
                                            self.code.selected_run,
                                            self.code.selected_instruction,
                                        );
                                let id = report.id;
                                let outcome = report.capture.outcome;
                                match self.session.append_disassembly(report) {
                                    Ok(index) => {
                                        self.refresh_execution_graph();
                                        self.dirty = true;
                                        self.history_dirty = true;
                                        if focus {
                                            self.attention.select(index);
                                            self.show_code = true;
                                            self.code.select_run(
                                                self.session.disassemblies.last().unwrap(),
                                            );
                                        }
                                        self.status = format!(
                                            "Run {}: {outcome:?}. Instructions, source mappings and raw output are recorded.",
                                            id.0
                                        );
                                        self.activity.push(self.status.clone());
                                    }
                                    Err(error) => self.error = Some(error),
                                }
                            }
                            Err(error) => {
                                self.error = Some(error);
                                self.status =
                                    "Disassembly could not start; current investigation retained."
                                        .into();
                            }
                        }
                    }
                    if self.close_after_job {
                        self.close_after_job = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.code_job = None;
                    self.error = Some(
                        "Disassembly worker stopped unexpectedly; existing results retained."
                            .into(),
                    );
                    if self.close_after_job {
                        self.close_after_job = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(50))
                }
            }
        }
        if let Some(receiver) = &self.artifact_worker {
            match receiver.try_recv() {
                Ok(result) => {
                    self.artifact_worker = None;
                    match result {
                        Ok(inspection) => {
                            if self.attach_source {
                                let matches = self.session.artifact.as_ref().is_some_and(|old| {
                                    old.sha256 == inspection.artifact.sha256
                                        && old.length == inspection.artifact.length
                                });
                                let image_matches =
                                    inspection.image.as_ref().is_some_and(|image| {
                                        self.session
                                            .artifact
                                            .as_ref()
                                            .and_then(|a| a.firmware.as_ref())
                                            .and_then(|f| f.application.as_ref())
                                            .is_some_and(|app| {
                                                image.map().image_id.as_str() == app.sha256
                                                    && image.map().mapped_base == app.mapped_base()
                                                    && image.map().image_length
                                                        == u64::from(app.entry().length)
                                            })
                                    });
                                let image_matches = image_matches
                                    || (self
                                        .session
                                        .artifact
                                        .as_ref()
                                        .is_some_and(|a| a.firmware.is_none())
                                        && inspection.image.is_none());
                                if matches && image_matches {
                                    self.image = inspection.image.map(Arc::new);
                                    self.imported_fresh = true;
                                    self.source_path = Some(inspection.path);
                                    self.status = "Original file and decoded image verified. This investigation is ready for a new bounded run.".into();
                                } else {
                                    self.error = Some("The selected source or decoded image does not match this saved investigation. Its existing records were retained.".into());
                                }
                            } else {
                                self.replace(
                                    Investigation::from_artifact(inspection.artifact),
                                    true,
                                );
                                self.image = inspection.image.map(Arc::new);
                                self.source_path = Some(inspection.path);
                                self.dirty = true;
                                self.history_dirty = true;
                                self.status = if self.show_data {
                                    "V15 package decoded. Open Code to disassemble a selected application span.".into()
                                } else {
                                    "Artifact identity recorded. Select its observation to inspect the bytes.".into()
                                };
                                self.activity.push(if self.show_data { "Completed exact V15 identity check, FWSC directory decode and application checks. No model or target execution was invoked.".into() }
                                    else { "Completed local file hashing and bounded byte preview.".into() });
                            }
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
                            RichText::new(format!("{} new", self.attention.unread()))
                                .color(ACCENT),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .selectable_label(self.attention.following, "Follow latest")
                            .clicked()
                        {
                            self.follow_latest();
                        }
                        if ui
                            .add_enabled(
                                self.attention.unread() > 0,
                                egui::Button::new("Review next"),
                            )
                            .clicked()
                        {
                            self.review_next();
                        }
                        ui.checkbox(&mut self.attention.held, "Presenter hold")
                            .on_hover_text("Hold this reading view while AI continues. Follow latest releases the hold.");
                        if self.dirty {
                            ui.label(RichText::new("Unsaved").small().color(MUTED));
                        }
                    });
                });
            });
        if self.engine.controller.is_some() {
            egui::Panel::top("execution_status").show(ui, |ui| {
                let action = crate::engine_view::EngineView::controls(ui, &self.engine);
                self.engine_action(action, &ctx);
            });
        }
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
        self.sync_jobs();
        if !ctx.egui_wants_keyboard_input() {
            self.capture_history();
        }
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
