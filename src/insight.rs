//! Terminal-style evidence inspection. Scalar views are interpretations, not recovered types.
use crate::graph::{ACCENT, kind_color};
use divulge::domain::{Investigation, Observation, Origin, RecordData, RecordId};
use eframe::egui::{self, Color32, FontId, RichText};

pub const SURFACE: Color32 = Color32::from_rgb(10, 15, 18);
const OUTPUT: Color32 = Color32::from_rgb(194, 211, 207);
const DIM: Color32 = Color32::from_rgb(115, 143, 143);
const AMBER: Color32 = Color32::from_rgb(224, 177, 113);

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ByteOrder {
    #[default]
    Little,
    Big,
}

#[derive(Default)]
pub struct Inspector {
    byte_record: Option<RecordId>,
    byte_index: usize,
    byte_order: ByteOrder,
}

#[derive(Default)]
pub struct Response {
    pub selected_record: Option<usize>,
    pub note_changed: bool,
}

impl Inspector {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        session: &mut Investigation,
        selected_index: usize,
        show_types: bool,
    ) -> Response {
        let record = session.records[selected_index].clone();
        let mut response = Response::default();
        let observation = match &record.data {
            RecordData::Observation(bytes) => Some((record.id, bytes.clone())),
            RecordData::Explanation { evidence, .. } => evidence.iter().find_map(|id| {
                session.record(*id).and_then(|source| match &source.data {
                    RecordData::Observation(bytes) => Some((*id, bytes.clone())),
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
                ui.label(mono(format!("#{:04}", record.id.0), DIM));
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

                if let Some((source_id, bytes)) = &observation {
                    self.bytes(ui, *source_id, bytes);
                } else {
                    ui.label(mono("[bytes] no observation attached to this record", DIM));
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(10.0);
                transcript(ui, "? question", &record.insight.question, ACCENT);
                transcript(ui, ":: purpose", &record.insight.purpose, DIM);
                transcript(
                    ui,
                    "> action",
                    &record.insight.action,
                    Color32::from_rgb(179, 164, 221),
                );
                transcript(ui, "< observation", &record.insight.observation, ACCENT);
                transcript(
                    ui,
                    "= explanation",
                    &record.insight.explanation,
                    Color32::from_rgb(218, 197, 133),
                );

                if let RecordData::Explanation { evidence, .. } = &record.data {
                    ui.label(mono("@ evidence", ACCENT));
                    for id in evidence {
                        if let Some(index) = session.records.iter().position(|r| r.id == *id)
                            && ui
                                .link(mono(
                                    format!("[{:04}] {}", id.0, session.records[index].title),
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

    fn bytes(&mut self, ui: &mut egui::Ui, id: RecordId, observation: &Observation) {
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
            "file",
            &format!(
                "0x{:08X}..0x{:08X} (end exclusive)",
                observation.offset.0,
                observation.offset.0 + observation.bytes.len() as u64
            ),
        );
        line(ui, "mapped", "unresolved / no load map");
        line(ui, "decode", "raw bytes / no instructions decoded");
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
                        let offset = observation.offset.0 + row as u64 * 8;
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
                                        "File offset 0x{:08X} / u8 {}",
                                        observation.offset.0 + index as u64,
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
        let offset = observation.offset.0 + self.byte_index as u64;
        let byte = observation.bytes[self.byte_index];
        ui.label(mono(format!("@ file+0x{offset:08X}"), ACCENT));
        ui.horizontal(|ui| {
            ui.label(mono("interpret as", DIM));
            ui.selectable_value(&mut self.byte_order, ByteOrder::Little, "little endian");
            ui.selectable_value(&mut self.byte_order, ByteOrder::Big, "big endian");
        });
        let u16_value =
            window::<2>(&observation.bytes, self.byte_index).map(|bytes| match self.byte_order {
                ByteOrder::Little => u16::from_le_bytes(bytes),
                ByteOrder::Big => u16::from_be_bytes(bytes),
            });
        let u32_value =
            window::<4>(&observation.bytes, self.byte_index).map(|bytes| match self.byte_order {
                ByteOrder::Little => u32::from_le_bytes(bytes),
                ByteOrder::Big => u32::from_be_bytes(bytes),
            });
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
                |bits| format!("{:e} / bits 0x{bits:08X}", f32::from_bits(bits)),
            ),
        );
        ui.label(mono("Chosen views, not recovered source types.", AMBER).size(10.0));
    }
}

fn window<const N: usize>(bytes: &[u8], start: usize) -> Option<[u8; N]> {
    bytes.get(start..start.checked_add(N)?)?.try_into().ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_view_requires_the_full_selected_range() {
        let bytes = [0xff, 0x34, 0x12];
        assert_eq!(window::<2>(&bytes, 1).map(u16::from_le_bytes), Some(4660));
        assert_eq!(window::<2>(&bytes, 1).map(u16::from_be_bytes), Some(13330));
        assert_eq!(window::<2>(&bytes, 2), None);
        assert_eq!(window::<4>(&bytes, 0), None);
        assert_eq!(window::<2>(&bytes, usize::MAX), None);
    }
}
