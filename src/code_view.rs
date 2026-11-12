use crate::graph::{ACCENT, MUTED, PANEL, TEXT};
use divulge::{
    domain::{Investigation, RecordData},
    pi32::{self, ControlFlow, Destination, DisassemblyReport, EdgeKind, Flow, RunId, Target},
    tool_runner::ToolConfig,
};
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2,
};

const AMBER: Color32 = Color32::from_rgb(224, 177, 113);
const BLOCK: Vec2 = Vec2::new(310.0, 142.0);

#[derive(Default)]
pub struct Action {
    pub request: Option<(u64, u64)>,
    pub cancel: bool,
    pub verify_source: bool,
    pub selected_run: Option<RunId>,
}

pub struct CodeView {
    pub config: ToolConfig,
    pub selected_run: Option<RunId>,
    pub selected_instruction: usize,
    offset: String,
    length: u64,
    show_flow: bool,
    error: Option<String>,
    flow: ControlFlow,
    bounds: Rect,
    reveal: bool,
}

impl Default for CodeView {
    fn default() -> Self {
        Self {
            config: ToolConfig::default(),
            selected_run: None,
            selected_instruction: 0,
            offset: "0x0".into(),
            length: 256,
            show_flow: false,
            error: None,
            flow: ControlFlow::default(),
            bounds: Rect::ZERO,
            reveal: false,
        }
    }
}

impl CodeView {
    pub fn stage(&mut self, offset: u64, length: u64) {
        self.offset = format!("0x{offset:X}");
        self.length = length;
    }
    pub fn select_run(&mut self, report: &DisassemblyReport) {
        if self.selected_run != Some(report.id) {
            self.selected_run = Some(report.id);
            self.selected_instruction = 0;
            self.flow = pi32::control_flow(report);
            self.focus_block();
            self.reveal = true;
        }
    }

    pub fn report<'a>(
        &self,
        session: &'a Investigation,
        visible: usize,
    ) -> Option<&'a DisassemblyReport> {
        let id = self.selected_run?;
        if !session
            .records
            .iter()
            .take(visible)
            .any(|r| matches!(r.data, RecordData::Disassembly(run) if run == id))
        {
            return None;
        }
        session.disassemblies.iter().find(|r| r.id == id)
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        session: &Investigation,
        visible: usize,
        fresh: bool,
        running: bool,
    ) -> Action {
        let mut action = Action::default();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("app.bin+").monospace());
            ui.add(
                egui::TextEdit::singleline(&mut self.offset)
                    .desired_width(90.0)
                    .hint_text("hex offset"),
            );
            ui.label("bytes");
            ui.add(
                egui::DragValue::new(&mut self.length)
                    .range(2..=4096)
                    .speed(2.0),
            );
            if ui
                .add_enabled(
                    fresh && !running && session.disassemblies.len() < 16,
                    egui::Button::new("Disassemble selection"),
                )
                .clicked()
            {
                match parse_offset(&self.offset) {
                    Ok(offset) => {
                        action.request = Some((offset, self.length));
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            if running {
                ui.spinner();
                if ui.button("Cancel").clicked() {
                    action.cancel = true;
                }
            }
        });
        if !fresh {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(
                        "Recorded results remain readable. Verify the source before a new run.",
                    )
                    .small()
                    .color(MUTED),
                );
                if ui
                    .add_enabled(
                        !running && session.artifact.is_some(),
                        egui::Button::new("Verify source..."),
                    )
                    .clicked()
                {
                    action.verify_source = true;
                }
            });
        }
        ui.collapsing("Tool / scope", |ui| {
            ui.label("Only the selected 2–4096 bytes are sent to the installed disassembler. An arbitrary offset may start inside an instruction.");
            ui.horizontal(|ui| { ui.label("WSL distribution"); ui.text_edit_singleline(&mut self.config.distribution); });
            ui.horizontal(|ui| { ui.label("objdump path"); ui.add(egui::TextEdit::singleline(&mut self.config.executable).hint_text("empty: discover installed Jieli tool")); });
            ui.label(RichText::new("pi32v2 / r3 / minstrev3 / hex immediates · 15 s tool limit · raw outputs retained").small().color(MUTED));
        });
        if let Some(error) = &self.error {
            ui.colored_label(AMBER, error);
        }
        let available: Vec<&DisassemblyReport> = session
            .disassemblies
            .iter()
            .filter(|report| {
                session
                    .records
                    .iter()
                    .take(visible)
                    .any(|r| matches!(r.data, RecordData::Disassembly(id) if id == report.id))
            })
            .collect();
        if self.report(session, visible).is_none()
            && let Some(latest) = available.last()
        {
            self.select_run(latest);
        }
        let mut chosen = self.selected_run;
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("code_run")
                .selected_text(
                    chosen.map_or_else(|| "No run yet".into(), |id| format!("Run {}", id.0)),
                )
                .show_ui(ui, |ui| {
                    for run in &available {
                        ui.selectable_value(
                            &mut chosen,
                            Some(run.id),
                            format!(
                                "Run {} / app+0x{:X} / {} B",
                                run.id.0, run.scope.offset.0, run.scope.length
                            ),
                        );
                    }
                });
            if ui
                .selectable_value(&mut self.show_flow, false, "Instructions")
                .clicked()
            {
                self.reveal = true;
            }
            ui.selectable_value(&mut self.show_flow, true, "Control flow");
            if self.show_flow {
                if ui.button("Focus block").clicked() {
                    self.focus_block();
                }
                if ui.button("Fit all").clicked() {
                    self.fit_flow();
                }
            }
        });
        if chosen != self.selected_run
            && let Some(report) = available.iter().find(|r| Some(r.id) == chosen)
        {
            self.select_run(report);
            action.selected_run = Some(report.id);
        }
        let Some(report) = self.report(session, visible) else {
            ui.add_space(24.0);
            ui.heading("From application bytes to instructions");
            ui.label("Choose a span and disassemble it. Each accepted row keeps its image offset, original file range, declared address and exact bytes.");
            ui.label(
                RichText::new("A recorded investigation never launches a tool during replay.")
                    .color(MUTED),
            );
            return action;
        };
        ui.label(
            RichText::new(format!(
                "{:?}  /  {} rows  /  {} of {} bytes decoded  /  {:?} coverage",
                report.capture.outcome,
                report.instructions.len(),
                report.decoded_bytes(),
                report.scope.length,
                report.coverage
            ))
            .monospace()
            .size(11.0)
            .color(ACCENT),
        );
        ui.label(RichText::new(if report.validated_semantics() { "Static candidates · branch profile validated with known-label fixtures · no execution observed" } else { "Static candidates · unvalidated tool profile; flow targets stay unresolved" }).small().color(MUTED));
        if !report.gaps.is_empty() || !report.issues.is_empty() {
            ui.collapsing(
                format!(
                    "{} byte gaps / {} parsing notes",
                    report.gaps.len(),
                    report.issues.len()
                ),
                |ui| {
                    for gap in &report.gaps {
                        ui.monospace(format!(
                            "app+0x{:X} / {} B / {}",
                            gap.span.offset.0,
                            gap.bytes.len(),
                            hex(&gap.bytes)
                        ));
                    }
                    for issue in &report.issues {
                        ui.label(issue);
                    }
                },
            );
        }
        ui.separator();
        if self.show_flow {
            self.graph(ui, report);
        } else {
            self.instructions(ui, report);
        }
        action
    }

    fn instructions(&mut self, ui: &mut egui::Ui, report: &DisassemblyReport) {
        ui.horizontal(|ui| {
            fixed(ui, "ADDRESS", 90.0, MUTED);
            fixed(ui, "FILE+", 82.0, MUTED);
            fixed(ui, "LEN", 28.0, MUTED);
            fixed(ui, "BYTES", 140.0, MUTED);
            fixed(ui, "INSTRUCTION", 160.0, MUTED);
        });
        let mut scroll = egui::ScrollArea::both()
            .id_salt(("code_rows", report.id.0))
            .auto_shrink([false, false]);
        if self.reveal {
            scroll = scroll.vertical_scroll_offset(self.selected_instruction as f32 * 25.0);
            self.reveal = false;
        }
        scroll.show_rows(ui, 25.0, report.instructions.len(), |ui, rows| {
            ui.set_min_width(640.0);
            for index in rows {
                let row = &report.instructions[index];
                ui.push_id(index, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add_sized(
                                [90.0, 22.0],
                                egui::Button::new(
                                    RichText::new(format!("{:08X}", row.address.0)).monospace(),
                                )
                                .selected(index == self.selected_instruction)
                                .frame(index == self.selected_instruction),
                            )
                            .clicked()
                        {
                            self.selected_instruction = index;
                        }
                        fixed(ui, &format!("{:08X}", row.source[0].offset.0), 82.0, MUTED);
                        fixed(ui, &row.width().to_string(), 28.0, MUTED);
                        fixed(ui, &hex(&row.bytes), 140.0, TEXT);
                        if ui
                            .add(
                                egui::Label::new(
                                    RichText::new(&row.text).monospace().size(12.0).color(
                                        if matches!(row.flow, Flow::Unresolved(_)) {
                                            AMBER
                                        } else {
                                            TEXT
                                        },
                                    ),
                                )
                                .sense(Sense::click())
                                .extend(),
                            )
                            .clicked()
                        {
                            self.selected_instruction = index;
                        }
                    });
                });
            }
        });
    }

    pub fn inspector(
        &mut self,
        ui: &mut egui::Ui,
        report: &DisassemblyReport,
        note: &mut String,
        types: bool,
    ) -> bool {
        ui.style_mut().override_font_id = Some(FontId::monospace(12.0));
        ui.visuals_mut().override_text_color = Some(TEXT);
        ui.label(
            RichText::new("HARNESS INSIGHT / INSTRUCTION")
                .color(ACCENT)
                .strong(),
        );
        ui.label(
            RichText::new(format!("recorded tool output / run {}", report.id.0))
                .small()
                .color(MUTED),
        );
        ui.separator();
        let mut changed = false;
        egui::ScrollArea::vertical().id_salt(("instruction_insight", report.id.0, self.selected_instruction)).auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(8.0);
            if let Some(row) = report.instructions.get(self.selected_instruction) {
                ui.label(RichText::new(format!("> {:08X}: {}", row.address.0, row.text)).color(ACCENT).size(14.0));
                line(ui, "declared", &format!("0x{:08X}", row.address.0));
                line(ui, "image", &format!("app.bin+0x{:08X}", row.span.offset.0));
                for span in &row.source { line(ui, "source", &format!("file+0x{:08X} / {} B", span.offset.0, span.length)); }
                line(ui, "bytes", &hex(&row.bytes));
                line(ui, "width", &format!("{} bytes", row.width()));
                line(ui, "mnemonic", row.mnemonic());
                line(ui, "operands", row.operands());
                for extra in &row.continuation { line(ui, "continued", extra.trim()); }
                if let Some(target) = row.target() {
                    line(ui, "target", &pi32::target_text(target));
                    if let Target::Resolved(address) = target {
                        let next_pc = row.address.0 + row.width() as u64;
                        line(ui, "derivation", &format!("0x{:08X} + {} + signed displacement -> 0x{:08X}", row.address.0, row.width(), address.0));
                        line(ui, "next PC", &format!("0x{next_pc:08X}"));
                        if let Some(index) = report.instructions.iter().position(|i| i.address == *address) {
                            if ui.link("Inspect target instruction").clicked() { self.selected_instruction = index; self.reveal = true; self.focus_block(); }
                        } else if address.0 >= report.map.mapped_base.0 && address.0 < report.map.mapped_base.0 + report.map.image_length {
                            let scope_start = report.map.mapped_base.0 + report.scope.offset.0;
                            let message = if (scope_start..scope_start + report.scope.length).contains(&address.0) {
                                "Target has no decoded instruction boundary in this selection."
                            } else {
                                "Destination is outside this decoded selection."
                            };
                            ui.label(RichText::new(message).color(AMBER));
                            if ui.button("Use target as next selection").clicked() {
                                let offset = address.0 - report.map.mapped_base.0;
                                self.offset = format!("0x{offset:X}"); self.length = 256.min(report.map.image_length - offset) & !1;
                            }
                        } else { ui.label(RichText::new("Destination is outside this application image.").color(AMBER)); }
                    }
                }
                ui.add_space(12.0); ui.separator(); ui.add_space(8.0);
                let (question, answer, next) = pi32::explanation(row);
                transcript(ui, "? question", question);
                transcript(ui, "= explanation", &answer);
                transcript(ui, ":: why this matters", "Instruction boundaries and destinations let us choose which block or value to inspect next. They do not establish a routine's purpose by themselves.");
                transcript(ui, "> next probe", next);
                transcript(ui, "! unresolved", "Runtime register values, taken branches and function roles have not been observed. These are static candidates from a bounded selection.");
                if types {
                    ui.collapsing("Rust types", |ui| {
                        ui.label("Instruction { span: ImageSpan, address: MappedAddress, bytes: Vec<u8>, flow: Flow, .. }");
                        ui.label("Flow::Conditional { target: Target, .. }; Target::Resolved | Unresolved");
                        ui.label("ToolOutcome and Coverage are independent: a failed run can retain useful partial evidence.");
                    });
                }
            } else { ui.label(RichText::new("No accepted instruction rows. Inspect the tool receipt and raw output below.").color(AMBER)); }
            ui.collapsing("Tool receipt / provenance", |ui| {
                line(ui, "outcome", &format!("{:?} / exit {:?} / {} ms", report.capture.outcome, report.capture.exit_code, report.capture.elapsed_ms));
                line(ui, "tool", &report.capture.executable); line(ui, "version", &report.capture.version);
                line(ui, "tool SHA", &report.capture.tool_sha256); line(ui, "flags", &report.capture.flags.join(" "));
                line(ui, "wrapper", "derived ELF32 / machine 241 / one .text section / never executed");
                line(ui, "ELF SHA", &report.wrapper_sha256); line(ui, "image SHA", report.map.image_id.as_str());
                line(ui, "source SHA", report.map.artifact_id.as_str());
                for limitation in &report.capture.limitations { ui.colored_label(AMBER, limitation); }
            });
            ui.collapsing(format!("Raw stdout / {} bytes", report.capture.stdout.len()), |ui| {
                if ui.button("Copy raw stdout as text").clicked() { ui.ctx().copy_text(String::from_utf8_lossy(&report.capture.stdout).into_owned()); }
                raw_output(ui, "stdout", &report.capture.stdout);
            });
            ui.collapsing(format!("Raw stderr / {} bytes", report.capture.stderr.len()), |ui| { raw_output(ui, "stderr", &report.capture.stderr); });
            if !report.capture.limitations.is_empty() { for limitation in &report.capture.limitations { ui.colored_label(AMBER, limitation); } }
            ui.add_space(10.0); ui.label(RichText::new("# analyst note / this run").color(ACCENT));
            changed = ui.add(egui::TextEdit::multiline(note).desired_width(f32::INFINITY).desired_rows(3)).changed();
        });
        changed
    }

    fn selected_block(&self) -> Option<usize> {
        self.flow
            .blocks
            .iter()
            .position(|b| (b.first..b.end).contains(&self.selected_instruction))
    }

    fn focus_block(&mut self) {
        let Some(selected) = self.selected_block() else {
            self.bounds = Rect::from_min_size(Pos2::ZERO, BLOCK);
            return;
        };
        // A distant call destination must not make the selected block unreadable.
        // The full projection remains available through Fit all and pan/zoom.
        self.bounds = block_rect(selected).expand2(egui::vec2(150.0, 100.0));
    }

    fn fit_flow(&mut self) {
        self.bounds = (0..self.flow.blocks.len())
            .fold(Rect::NOTHING, |bounds, index| {
                bounds.union(block_rect(index))
            })
            .expand(40.0);
    }

    fn graph(&mut self, ui: &mut egui::Ui, report: &DisassemblyReport) {
        ui.label(RichText::new(format!("{} basic blocks · click a block to inspect its first instruction · arrows are static candidates", self.flow.blocks.len())).small().color(MUTED));
        if self.flow.blocks.is_empty() {
            ui.label("No decoded blocks in this result.");
            return;
        }
        let selected = self.selected_block();
        let mut clicked = None;
        egui::Scene::new()
            .zoom_range(0.15..=1.6)
            .show(ui, &mut self.bounds, |ui| {
                let painter = ui.painter().clone();
                for edge in &self.flow.edges {
                    let Destination::Block(to) = edge.to else {
                        continue;
                    };
                    let from_rect = block_rect(edge.from);
                    let to_rect = block_rect(to);
                    let (start, end) = if to_rect.top() > from_rect.bottom() {
                        (from_rect.center_bottom(), to_rect.center_top())
                    } else if to_rect.bottom() < from_rect.top() {
                        (from_rect.center_top(), to_rect.center_bottom())
                    } else if to_rect.left() > from_rect.right() {
                        (from_rect.right_center(), to_rect.left_center())
                    } else {
                        (from_rect.left_center(), to_rect.right_center())
                    };
                    let color = edge_color(edge.kind);
                    if to == edge.from {
                        let outside = from_rect.right_top() + egui::vec2(30.0, -20.0);
                        painter.line_segment([start, outside], Stroke::new(1.5, color));
                        painter.line_segment(
                            [outside, from_rect.center_top()],
                            Stroke::new(1.5, color),
                        );
                    } else {
                        arrow(&painter, start, end, color);
                    }
                    painter.text(
                        start.lerp(end, 0.45),
                        Align2::CENTER_BOTTOM,
                        edge.kind.label(),
                        FontId::monospace(10.0),
                        color,
                    );
                }
                for (index, block) in self.flow.blocks.iter().enumerate() {
                    let rect = block_rect(index);
                    ui.expand_to_include_rect(rect);
                    let active = selected == Some(index);
                    painter.rect(
                        rect,
                        6.0,
                        if active {
                            Color32::from_rgb(28, 47, 49)
                        } else {
                            PANEL
                        },
                        Stroke::new(
                            if active { 1.8 } else { 1.0 },
                            if active {
                                ACCENT
                            } else {
                                Color32::from_rgb(62, 75, 87)
                            },
                        ),
                        StrokeKind::Inside,
                    );
                    painter.text(
                        rect.min + egui::vec2(12.0, 10.0),
                        Align2::LEFT_TOP,
                        format!(
                            "B{index:02}  0x{:08X}  / {} {}",
                            block.address.0,
                            block.end - block.first,
                            if block.end - block.first == 1 {
                                "instruction"
                            } else {
                                "instructions"
                            }
                        ),
                        FontId::monospace(12.0),
                        ACCENT,
                    );
                    for (line, row) in report.instructions[block.first..block.end]
                        .iter()
                        .take(3)
                        .enumerate()
                    {
                        painter.text(
                            rect.min + egui::vec2(12.0, 35.0 + line as f32 * 19.0),
                            Align2::LEFT_TOP,
                            truncate(&format!("{:08X}  {}", row.address.0, row.text), 40),
                            FontId::monospace(11.0),
                            TEXT,
                        );
                    }
                    if block.end - block.first > 3 {
                        painter.text(
                            rect.min + egui::vec2(12.0, 94.0),
                            Align2::LEFT_TOP,
                            format!("... {} more", block.end - block.first - 3),
                            FontId::monospace(10.0),
                            MUTED,
                        );
                    }
                    let exits: Vec<String> = self
                        .flow
                        .edges
                        .iter()
                        .filter(|edge| edge.from == index)
                        .filter_map(|edge| match &edge.to {
                            Destination::Block(_) => None,
                            Destination::Outside(address) => Some(format!(
                                "{} 0x{:08X} / outside selection",
                                edge.kind.label(),
                                address.0
                            )),
                            Destination::Unresolved(_) => Some("unresolved exit".into()),
                            Destination::Return => Some("return / caller unknown".into()),
                        })
                        .collect();
                    painter.text(
                        rect.min + egui::vec2(12.0, 119.0),
                        Align2::LEFT_TOP,
                        truncate(&exits.join(" · "), 43),
                        FontId::monospace(10.0),
                        AMBER,
                    );
                    if ui
                        .interact(
                            rect,
                            egui::Id::new(("basic_block", report.id.0, index)),
                            Sense::click(),
                        )
                        .clicked()
                    {
                        clicked = Some(block.first);
                    }
                }
            });
        if let Some(index) = clicked {
            self.selected_instruction = index;
            self.reveal = true;
        }
    }
}

fn parse_offset(value: &str) -> Result<u64, String> {
    let value = value.trim();
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16)
    } else {
        value.parse()
    }
    .map_err(|_| "Use a hexadecimal image offset such as 0x120, or a decimal byte offset".into())
}
fn block_rect(index: usize) -> Rect {
    Rect::from_min_size(
        Pos2::new((index % 2) as f32 * 370.0, (index / 2) as f32 * 195.0),
        BLOCK,
    )
}
fn edge_color(kind: EdgeKind) -> Color32 {
    match kind {
        EdgeKind::Taken | EdgeKind::Jump => ACCENT,
        EdgeKind::Call => Color32::from_rgb(185, 158, 228),
        EdgeKind::Unknown => AMBER,
        _ => Color32::from_rgb(134, 167, 202),
    }
}
fn arrow(painter: &egui::Painter, start: Pos2, end: Pos2, color: Color32) {
    painter.line_segment([start, end], Stroke::new(1.4, color));
    let direction = (end - start).normalized();
    let side = egui::vec2(-direction.y, direction.x);
    painter.line_segment(
        [end - direction * 8.0 + side * 4.0, end],
        Stroke::new(1.4, color),
    );
    painter.line_segment(
        [end - direction * 8.0 - side * 4.0, end],
        Stroke::new(1.4, color),
    );
}
fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
fn truncate(text: &str, length: usize) -> String {
    let mut result: String = text.chars().take(length).collect();
    if text.chars().count() > length {
        result.push('…');
    }
    result
}
fn fixed(ui: &mut egui::Ui, value: &str, width: f32, color: Color32) {
    ui.add_sized(
        [width, 22.0],
        egui::Label::new(RichText::new(value).monospace().size(11.0).color(color)),
    );
}
fn line(ui: &mut egui::Ui, name: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{name:<10}")).color(MUTED));
        ui.add(egui::Label::new(value).wrap().selectable(true));
    });
}
fn transcript(ui: &mut egui::Ui, name: &str, value: &str) {
    ui.label(RichText::new(name).color(ACCENT));
    ui.add(egui::Label::new(value).wrap().selectable(true));
    ui.add_space(9.0);
}
fn raw_output(ui: &mut egui::Ui, salt: &str, bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text.lines().collect();
    egui::ScrollArea::both()
        .id_salt(salt)
        .max_height(240.0)
        .show_rows(ui, 17.0, lines.len(), |ui, visible| {
            for index in visible {
                ui.add(
                    egui::Label::new(RichText::new(lines[index]).monospace().size(10.0))
                        .extend()
                        .selectable(true),
                );
            }
        });
}
