//! Terminal-style evidence inspection. Scalar views are interpretations, not recovered types.
use crate::data_view::ByteView;
use crate::graph::{ACCENT, kind_color};
use divulge::domain::{
    InsightId, Investigation, ObservationId, Origin, QuestionPlan, RecordData, RecordId,
};
use divulge::evidence::{ByteOrder, ScalarViews, TypeHypothesis};
use eframe::egui::{self, Color32, FontId, RichText};

pub const SURFACE: Color32 = Color32::from_rgb(10, 15, 18);
const OUTPUT: Color32 = Color32::from_rgb(194, 211, 207);
const DIM: Color32 = Color32::from_rgb(115, 143, 143);
const AMBER: Color32 = Color32::from_rgb(224, 177, 113);

#[derive(Default)]
pub struct Inspector {
    byte_record: Option<RecordId>,
    byte_index: usize,
    byte_order: ByteOrder,
    hypothesis_record: Option<RecordId>,
    proposed_type: String,
    rationale: String,
    limitation: String,
    counter_evidence: Vec<ObservationId>,
    hypothesis_error: String,
}

#[derive(Default)]
pub struct Response {
    pub selected_record: Option<usize>,
    pub note_changed: bool,
    pub created_record: Option<usize>,
}

impl Inspector {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        session: &mut Investigation,
        selected_index: usize,
        visible_count: usize,
        show_types: bool,
    ) -> Response {
        let record = session.records[selected_index].clone();
        let question = session.question_for(record.id).cloned();
        let owns_question = question.as_ref().is_some_and(|q| q.id == record.id);
        let mut response = Response::default();
        let firmware_view = match &record.data {
            RecordData::Firmware(view) => Some(*view),
            RecordData::Explanation { evidence, .. } => evidence.iter().find_map(|id| {
                session
                    .record(id.record())
                    .and_then(|source| match source.data {
                        RecordData::Firmware(view) => Some(view),
                        _ => None,
                    })
            }),
            _ => None,
        };
        let observation = match &record.data {
            RecordData::Observation(bytes) => Some((record.id, bytes.clone())),
            RecordData::Explanation { evidence, .. } => evidence.iter().find_map(|id| {
                session
                    .record(id.record())
                    .and_then(|source| match &source.data {
                        RecordData::Observation(bytes) => Some((id.record(), bytes.clone())),
                        _ => None,
                    })
            }),
            _ => None,
        };

        ui.style_mut().override_font_id = Some(FontId::monospace(12.0));
        ui.spacing_mut().item_spacing = egui::vec2(5.0, 3.0);
        ui.visuals_mut().override_text_color = Some(OUTPUT);
        ui.visuals_mut().text_edit_bg_color = Some(Color32::from_rgb(16, 25, 28));

        ui.horizontal(|ui| {
            ui.label(mono("HARNESS INSIGHT", ACCENT).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(mono(
                    format!("#{:04}", InsightId::new(record.id.0).unwrap().value()),
                    DIM,
                ));
            });
        });
        ui.label(mono(
            match session.origin {
                Origin::WorkshopFixture => "fixture / recorded output",
                Origin::LocalArtifact => "artifact / recorded output",
            },
            DIM,
        ));
        ui.add_space(7.0);
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt(("insight", record.id.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.label(mono(format!("> {}", record.title), ACCENT).size(14.0));
                let qualifier = match &record.data {
                    RecordData::Explanation { basis, .. } => format!(" / {basis}"),
                    _ => String::new(),
                };
                ui.label(mono(
                    format!("[{}{}]", record.data.label().to_lowercase(), qualifier),
                    kind_color(&record.data),
                ));
                ui.add_space(10.0);

                if let Some(question) = &question {
                    let plan = question.question_plan.as_ref().unwrap();
                    if owns_question {
                        transcript(
                            ui,
                            "? what are we trying to find out?",
                            &question.insight.question,
                            ACCENT,
                        );
                        question_plan(ui, plan);
                        ui.label(mono("-> probes and responses", ACCENT));
                        let mut linked_count = 0;
                        for (index, related) in
                            session.records.iter().take(visible_count).enumerate()
                        {
                            if related.question_id.is_some_and(|id|id.record()==question.id) {
                                linked_count += 1;
                                if ui
                                    .link(mono(
                                        format!(
                                            "[{:04}] {} / {}",
                                            related.id.0,
                                            related.data.label().to_lowercase(),
                                            related.title
                                        ),
                                        kind_color(&related.data),
                                    ))
                                    .clicked()
                                {
                                    response.selected_record = Some(index);
                                }
                            }
                        }
                        if linked_count == 0 {
                            ui.label(mono("No probe result is available at this step.", DIM));
                        }
                    } else {
                        egui::CollapsingHeader::new(mono(
                            format!("? question #{:04} / why, how, to what end", question.id.0),
                            ACCENT,
                        ))
                        .id_salt(("question_context", record.id.0))
                        .show(ui, |ui| {
                            transcript(ui, "question", &question.insight.question, ACCENT);
                            question_plan(ui, plan);
                            if ui.link(mono("Open this question", ACCENT)).clicked() {
                                response.selected_record =
                                    session.records.iter().position(|r| r.id == question.id);
                            }
                        });
                    }
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);
                } else if matches!(
                    record.data,
                    RecordData::Question | RecordData::Unknown { .. }
                ) {
                    transcript(ui, "? question", &record.insight.question, ACCENT);
                    ui.label(mono(
                        "No detailed question plan was saved with this record.",
                        AMBER,
                    ));
                    ui.add_space(10.0);
                }

                if let Some(artifact) = &session.artifact {
                    line(ui, "file", &artifact.name);
                    line(
                        ui,
                        "size",
                        &format!("{} B / 0x{:08X}", artifact.length, artifact.length),
                    );
                    ui.horizontal(|ui| {
                        ui.label(mono("sha256", DIM));
                        let short =
                            format!("{}..{}", &artifact.sha256[..12], &artifact.sha256[56..]);
                        if ui
                            .link(mono(short, ACCENT))
                            .on_hover_text(&artifact.sha256)
                            .clicked()
                        {
                            ui.ctx().copy_text(artifact.sha256.clone());
                        }
                    });
                    ui.label(mono("Hash: click to copy / hover for full value", DIM).size(10.0));
                } else {
                    line(ui, "source", "synthetic two-byte teaching fixture");
                }
                ui.add_space(9.0);

                if let Some(view) = firmware_view
                    && let Some(report) =
                        session.artifact.as_ref().and_then(|a| a.firmware.as_ref())
                {
                    crate::data_view::fields(ui, report, view);
                    self.bytes(ui, record.id, &crate::data_view::bytes(report, view));
                } else if let Some((source_id, bytes)) = &observation {
                    self.bytes(
                        ui,
                        *source_id,
                        &ByteView {
                            bytes: &bytes.bytes,
                            offset: bytes.offset.0,
                            space: "file",
                            source: vec![divulge::fwsc::SourceSpan {
                                offset: bytes.offset,
                                length: bytes.bytes.len() as u64,
                            }],
                            mapped: None,
                            transform: "identity / raw bytes / no instructions decoded",
                        },
                    );
                } else {
                    ui.label(mono("[bytes] no observation attached to this record", DIM));
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(10.0);
                if !owns_question {
                    transcript(ui, "? question", &record.insight.question, ACCENT);
                    transcript(ui, ":: purpose", &record.insight.purpose, DIM);
                    transcript(
                        ui,
                        "> action",
                        &record.insight.action,
                        Color32::from_rgb(179, 164, 221),
                    );
                }
                transcript(ui, "< observation", &record.insight.observation, ACCENT);
                transcript(
                    ui,
                    "= explanation",
                    &record.insight.explanation,
                    Color32::from_rgb(218, 197, 133),
                );

                if let RecordData::TypeHypothesis(hypothesis)=&record.data {
                    ui.label(mono(format!("Proposed target type: {} / inference",hypothesis.proposed_type),AMBER));
                    for (label,ids) in [("supporting evidence",&hypothesis.evidence),("counter-evidence",&hypothesis.counter_evidence)]{
                        ui.label(mono(label,DIM));
                        if ids.is_empty(){ui.label("None recorded; this does not prove absence.");}
                        for id in ids { if let Some(index)=session.records.iter().take(visible_count).position(|r|r.id==id.record()) && ui.link(format!("#{} / {}",id.value(),session.records[index].title)).clicked(){response.selected_record=Some(index);} }
                    }
                }
                if let RecordData::Explanation { evidence, .. } = &record.data {
                    ui.label(mono("@ evidence", ACCENT));
                    for id in evidence {
                        if let Some(index) = session.records.iter().position(|r| r.id == id.record())
                            && ui
                                .link(mono(
                                    format!("[{:04}] {}", id.value(), session.records[index].title),
                                    ACCENT,
                                ))
                                .clicked()
                        {
                            response.selected_record = Some(index);
                        }
                    }
                    ui.add_space(10.0);
                }
                transcript(ui, "! unresolved", &record.insight.uncertainty, AMBER);
                transcript(ui, "> next probe", &record.insight.next_step, ACCENT);
                if let Some(run) = session.ai_runs.iter().find(|run| run.record == record.id) {
                    ui.label(mono("@ OMP / proposal receipt", ACCENT));
                    if !run.diagnostic.is_empty() {
                        ui.label(mono(&run.diagnostic, AMBER));
                    }
                    ui.label(mono(
                        format!("{} / {} / probe not executed", run.id, run.outcome),
                        DIM,
                    ));
                    if let Some(proposal) = &run.proposal {
                        for id in &proposal.evidence {
                            if let Some(index) = session.records.iter().position(|r| r.id == id.record())
                                && ui
                                    .link(mono(format!("Evidence #{:04}", id.value()), ACCENT))
                                    .clicked()
                            {
                                response.selected_record = Some(index);
                            }
                        }
                    }
                    ui.collapsing("Model / isolation / recorded transcript", |ui| {
                        ui.label(mono(format!("Context SHA-256 {}", run.context_sha256), DIM));
                        for event in &run.events {
                            if event.kind == "ready" {
                                ui.label(mono(
                                    format!(
                                        "Model {} / SDK {} / CLI {} / tools {}",
                                        event.data["selected"],
                                        event.data["sdk_version"],
                                        event.data["cli_version"],
                                        event.data["active_tools"]
                                    ),
                                    OUTPUT,
                                ));
                            } else if event.kind != "draft" {
                                ui.add(
                                    egui::Label::new(mono(
                                        format!("{} {}", event.kind, event.data),
                                        OUTPUT,
                                    ))
                                    .wrap(),
                                );
                            }
                        }
                    });
                    ui.collapsing("Exact supplied evidence snapshot", |ui| {
                        ui.add(
                            egui::Label::new(mono(
                                serde_json::to_string_pretty(&run.context).unwrap(),
                                DIM,
                            ))
                            .wrap(),
                        );
                    });
                }
                if let Some(run) = session.rea_runs.iter().find(|run| run.record == record.id) {
                    crate::rea_view::receipt(ui, run);
                }
                if show_types {
                    ui.collapsing(mono("types / record schema", DIM), |ui| {
                        ui.add(
                            egui::Label::new(mono(record.data.rust_type(), OUTPUT))
                                .wrap()
                                .selectable(true),
                        );
                        ui.label(mono(
                            "RecordId links evidence; graph position is presentation state.",
                            DIM,
                        ));
                    });
                }
                ui.add_space(12.0);
                if session.is_evidence(&record) {
                    if self.hypothesis_record!=Some(record.id) {
                        self.hypothesis_record=Some(record.id); self.proposed_type.clear(); self.rationale.clear(); self.counter_evidence.clear(); self.hypothesis_error.clear();
                        self.limitation="A chosen interpretation does not establish an original source type, packing or runtime use.".into();
                    }
                    ui.collapsing("Record a type hypothesis",|ui|{
                        ui.label("Proposed target type");ui.text_edit_singleline(&mut self.proposed_type);
                        ui.label("Why these observations suggest it");ui.text_edit_multiline(&mut self.rationale);
                        ui.label("What remains unproved");ui.text_edit_multiline(&mut self.limitation);
                        ui.label(format!("Supporting evidence: #{}",record.id.0));
                        ui.collapsing("Counter-evidence",|ui|{for candidate in session.records.iter().take(visible_count).filter(|r|session.is_evidence(r)&&r.id!=record.id){let id=ObservationId::new(candidate.id.0).unwrap();let mut checked=self.counter_evidence.contains(&id);if ui.checkbox(&mut checked,format!("#{} {}",id.value(),candidate.title)).changed(){if checked {self.counter_evidence.push(id);}else{self.counter_evidence.retain(|i|i!=&id);}}}});
                        if ui.add_enabled(!self.proposed_type.trim().is_empty()&&!self.rationale.trim().is_empty()&&!self.limitation.trim().is_empty(),egui::Button::new("Record hypothesis with evidence")).clicked(){
                            match session.append_type_hypothesis(TypeHypothesis{proposed_type:self.proposed_type.clone(),rationale:self.rationale.clone(),evidence:vec![ObservationId::new(record.id.0).unwrap()],counter_evidence:self.counter_evidence.clone(),limitation:self.limitation.clone()}){Ok(index)=>response.created_record=Some(index),Err(error)=>self.hypothesis_error=error}
                        }
                        if !self.hypothesis_error.is_empty(){ui.label(&self.hypothesis_error);}
                    });
                }
                ui.label(mono("# analyst note", ACCENT));
                response.note_changed = ui
                    .add(
                        egui::TextEdit::multiline(&mut session.records[selected_index].note)
                            .font(FontId::monospace(12.0))
                            .hint_text("Prediction, correction, next question...")
                            .desired_rows(3)
                            .desired_width(f32::INFINITY),
                    )
                    .changed();
            });
        response
    }

    fn bytes(&mut self, ui: &mut egui::Ui, id: RecordId, observation: &ByteView<'_>) {
        if self.byte_record != Some(id) {
            self.byte_record = Some(id);
            self.byte_index = 0;
        }
        self.byte_index = self
            .byte_index
            .min(observation.bytes.len().saturating_sub(1));
        ui.label(mono(format!("[bytes / evidence #{:04}]", id.0), ACCENT));
        line(
            ui,
            observation.space,
            &format!(
                "0x{:08X}..0x{:08X} (end exclusive)",
                observation.offset,
                observation.offset + observation.bytes.len() as u64
            ),
        );
        for source in &observation.source {
            line(
                ui,
                "source",
                &format!("file+0x{:08X} / {} B", source.offset.0, source.length),
            );
        }
        line(
            ui,
            "mapped",
            &observation.mapped.map_or_else(
                || "unresolved / no load map".into(),
                |base| format!("0x{base:08X} / declared, not runtime-observed"),
            ),
        );
        line(ui, "decode", observation.transform);
        ui.add_space(8.0);
        ui.label(mono("OFFSET    HEX                      ASCII", DIM).size(11.0));
        egui::ScrollArea::vertical()
            .id_salt(("observed_bytes", id.0))
            .max_height(154.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                ui.spacing_mut().button_padding = egui::vec2(1.0, 1.0);
                for (row, chunk) in observation.bytes.chunks(8).enumerate() {
                    ui.horizontal(|ui| {
                        let offset = observation.offset + row as u64 * 8;
                        ui.label(mono(format!("{offset:08X}"), DIM).size(11.0));
                        ui.add_space(4.0);
                        for column in 0..8 {
                            let index = row * 8 + column;
                            if let Some(byte) = chunk.get(column) {
                                let selected = self.byte_index == index;
                                if ui
                                    .add_sized(
                                        [20.0, 18.0],
                                        egui::Button::new(
                                            mono(
                                                format!("{byte:02X}"),
                                                if selected { ACCENT } else { OUTPUT },
                                            )
                                            .size(11.0),
                                        )
                                        .selected(selected)
                                        .frame(selected),
                                    )
                                    .on_hover_text(format!(
                                        "{} offset 0x{:08X} / u8 {}",
                                        observation.space,
                                        observation.offset + index as u64,
                                        byte
                                    ))
                                    .clicked()
                                {
                                    self.byte_index = index;
                                }
                            } else {
                                ui.allocate_space(egui::vec2(20.0, 18.0));
                            }
                        }
                        ui.add_space(5.0);
                        let ascii: String = chunk
                            .iter()
                            .map(|byte| {
                                if byte.is_ascii_graphic() || *byte == b' ' {
                                    *byte as char
                                } else {
                                    '.'
                                }
                            })
                            .collect();
                        ui.label(mono(ascii, DIM).size(11.0));
                    });
                }
            });
        ui.add_space(8.0);
        if observation.bytes.is_empty() {
            ui.label(mono("[empty] no bytes returned", DIM));
            return;
        }
        let offset = observation.offset + self.byte_index as u64;
        let byte = observation.bytes[self.byte_index];
        ui.label(mono(
            format!("@ {}+0x{offset:08X}", observation.space),
            ACCENT,
        ));
        if let Some(base) = observation.mapped {
            line(
                ui,
                "declared",
                &format!("0x{:08X}", base + self.byte_index as u64),
            );
        }
        ui.horizontal(|ui| {
            ui.label(mono("interpret as", DIM));
            ui.selectable_value(&mut self.byte_order, ByteOrder::Little, "little endian");
            ui.selectable_value(&mut self.byte_order, ByteOrder::Big, "big endian");
        });
        let values = ScalarViews::from_bytes(observation.bytes, self.byte_index, self.byte_order);
        let u16_value = values.u16_value;
        let u32_value = values.u32_value;
        line(
            ui,
            "u8 / i8",
            &format!("{byte} / {} / 0b{byte:08b}", byte as i8),
        );
        line(
            ui,
            "u16",
            &u16_value.map_or_else(
                || "-- / need 2 observed bytes".into(),
                |value| format!("{value} / 0x{value:04X}"),
            ),
        );
        line(
            ui,
            "u32",
            &u32_value.map_or_else(
                || "-- / need 4 observed bytes".into(),
                |value| format!("{value} / 0x{value:08X}"),
            ),
        );
        line(
            ui,
            "f32",
            &u32_value.map_or_else(
                || "-- / need 4 observed bytes".into(),
                |bits| {
                    format!(
                        "{:e} / bits 0x{bits:08X}",
                        divulge::evidence::F32Bits(bits).value()
                    )
                },
            ),
        );
        ui.label(mono("Chosen views, not recovered source types.", AMBER).size(10.0));
    }
}

fn mono(text: impl Into<String>, color: Color32) -> RichText {
    RichText::new(text).monospace().size(12.0).color(color)
}

fn line(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(mono(format!("{label:<9}"), DIM));
        ui.add(
            egui::Label::new(mono(value, OUTPUT))
                .wrap()
                .selectable(true),
        );
    });
}

fn transcript(ui: &mut egui::Ui, prompt: &str, body: &str, color: Color32) {
    ui.label(mono(prompt, color).strong());
    ui.add(egui::Label::new(mono(body, OUTPUT)).wrap().selectable(true));
    ui.add_space(10.0);
}

fn question_plan(ui: &mut egui::Ui, plan: &QuestionPlan) {
    transcript(
        ui,
        "WHY / why ask this?",
        &plan.why,
        Color32::from_rgb(138, 178, 239),
    );
    ui.label(
        mono(
            "HOW / how will we answer?",
            Color32::from_rgb(179, 164, 221),
        )
        .strong(),
    );
    for (index, step) in plan.steps.iter().enumerate() {
        ui.add(
            egui::Label::new(mono(format!("{}. {step}", index + 1), OUTPUT))
                .wrap()
                .selectable(true),
        );
        ui.add_space(4.0);
    }
    ui.add_space(6.0);
    transcript(
        ui,
        "TO WHAT END / what does this enable?",
        &plan.enables,
        ACCENT,
    );
    transcript(
        ui,
        "ANSWERED WHEN / what evidence is enough?",
        &plan.answered_when,
        Color32::from_rgb(218, 197, 133),
    );
    transcript(
        ui,
        "IF NOT / what changes our next step?",
        &plan.if_not,
        AMBER,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_view_requires_the_full_selected_range() {
        let bytes = [0xff, 0x34, 0x12];
        assert_eq!(
            ScalarViews::from_bytes(&bytes, 1, ByteOrder::Little).u16_value,
            Some(4660)
        );
        assert_eq!(
            ScalarViews::from_bytes(&bytes, 1, ByteOrder::Big).u16_value,
            Some(13330)
        );
        assert_eq!(
            ScalarViews::from_bytes(&bytes, 2, ByteOrder::Little).u16_value,
            None
        );
        assert_eq!(
            ScalarViews::from_bytes(&bytes, 0, ByteOrder::Little).u32_value,
            None
        );
        assert_eq!(
            ScalarViews::from_bytes(&bytes, usize::MAX, ByteOrder::Little).u16_value,
            None
        );
    }
}
