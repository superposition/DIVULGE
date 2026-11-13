//! Native egui projection of exact per-region byte distributions.
use crate::graph::{ACCENT, MUTED};
use divulge::byte_matrix::ByteMatrix;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};

pub struct MatrixView {
    yaw: f32,
    pitch: f32,
    zoom: f32,
    height: f32,
    selected_row: usize,
    selected_byte: usize,
}

impl Default for MatrixView {
    fn default() -> Self {
        Self {
            yaw: -0.65,
            pitch: 0.8,
            zoom: 1.0,
            height: 1.0,
            selected_row: 0,
            selected_byte: 0,
        }
    }
}

fn entropy_color(entropy: f64) -> Color32 {
    let t = (entropy / 8.0).clamp(0.0, 1.0) as f32;
    Color32::from_rgb(
        (55.0 + 177.0 * t) as u8,
        (178.0 - 21.0 * t) as u8,
        (198.0 - 107.0 * t) as u8,
    )
}

fn camera(point: [f32; 3], yaw: f32, pitch: f32) -> (Pos2, f32) {
    let (s, c) = yaw.sin_cos();
    let u = point[0] * c - point[1] * s;
    let v = point[0] * s + point[1] * c;
    let (s, c) = pitch.sin_cos();
    let depth = v * c + point[2] * s;
    let perspective = 3.4 / (3.4 - depth * 0.3).max(0.8);
    (Pos2::new(u, v * s - point[2] * c) * perspective, depth)
}

fn camera_bounds(yaw: f32, pitch: f32, height: f32) -> Rect {
    let mut bounds = Rect::NOTHING;
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [0.0, height] {
                bounds.extend_with(camera([x, y, z], yaw, pitch).0);
            }
        }
    }
    bounds
}

fn project(
    point: [f32; 3],
    yaw: f32,
    pitch: f32,
    rect: Rect,
    zoom: f32,
    bounds: Rect,
) -> (Pos2, f32) {
    let (point, depth) = camera(point, yaw, pitch);
    let size = rect.shrink(32.0).size().max(Vec2::splat(1.0));
    let scale = (size.x / bounds.width()).min(size.y / bounds.height()) * zoom;
    (rect.center() + (point - bounds.center()) * scale, depth)
}

impl MatrixView {
    pub fn show(&mut self, ui: &mut egui::Ui, data: &ByteMatrix, source: &str, coverage: &str) {
        ui.heading("Source byte matrix / 3D");
        ui.label(RichText::new(source).monospace().color(ACCENT));
        ui.label(RichText::new(coverage).small().color(MUTED));
        ui.label(
            "X: byte value 00–FF   Y: consecutive source regions   Z: frequency within each region",
        );
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(entropy_color(0.0), "Color: 0 bits/byte");
            ui.label("→");
            ui.colored_label(entropy_color(8.0), "8 bits/byte");
            ui.label(
                "Shannon entropy; high entropy alone does not establish encryption or compression.",
            );
        });
        if data.windows.is_empty() {
            ui.label("No source bytes are available for this view.");
            return;
        }
        self.selected_row = self.selected_row.min(data.windows.len() - 1);
        self.selected_byte = self.selected_byte.min(255);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Reset camera").clicked() {
                self.yaw = -0.65;
                self.pitch = 0.8;
                self.zoom = 1.0;
                self.height = 1.0;
            }
            ui.add(egui::Slider::new(&mut self.zoom, 0.5..=4.0).text("Zoom"));
            ui.add(egui::Slider::new(&mut self.height, 0.3..=2.0).text("Height"));
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::Slider::new(&mut self.yaw, -std::f32::consts::PI..=std::f32::consts::PI)
                    .text("Rotate"),
            );
            ui.add(egui::Slider::new(&mut self.pitch, 0.15..=1.5).text("Tilt"));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Inspect region");
            ui.add(egui::DragValue::new(&mut self.selected_row).range(0..=data.windows.len() - 1));
            ui.label("byte (decimal)");
            ui.add(egui::DragValue::new(&mut self.selected_byte).range(0..=255));
            ui.label("Drag the plot to rotate. Click a point to inspect it.");
        });
        let row = &data.windows[self.selected_row];
        let count = row.counts[self.selected_byte];
        let selection = format!(
            "SOURCE +0x{:08X}..+0x{:08X} (end exclusive) · {} bytes\nByte 0x{:02X}: {} occurrences / {:.3}% · region entropy {:.4} bits/byte",
            row.offset,
            row.offset + row.length as u64,
            row.length,
            self.selected_byte,
            count,
            100.0 * f64::from(count) / row.length as f64,
            row.entropy
        );
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new(&selection).monospace().color(ACCENT));
            ui.horizontal_wrapped(|ui| {
                if ui.button("Copy selected range").clicked() {
                    ui.ctx().copy_text(selection.clone());
                }
                ui.label(
                    RichText::new(format!(
                        "Region starts: {}",
                        row.preview
                            .iter()
                            .map(|byte| format!("{byte:02X}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    ))
                    .monospace()
                    .small(),
                );
            });
        });
        let size = Vec2::new(
            ui.available_width().max(1.0),
            ui.available_height().max(220.0),
        );
        let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 4.0, Color32::from_rgb(12, 18, 24));
        if response.dragged() {
            let delta = ui.input(|i| i.pointer.delta());
            self.yaw = (self.yaw + delta.x * 0.008 + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.pitch = (self.pitch - delta.y * 0.006).clamp(0.15, 1.5);
        }
        let bounds = camera_bounds(self.yaw, self.pitch, self.height);
        let project = |point| project(point, self.yaw, self.pitch, rect, self.zoom, bounds);
        let grid = Stroke::new(1.0, Color32::from_rgb(40, 58, 68));
        for step in 0..=8 {
            let value = step as f32 * 0.25 - 1.0;
            painter.line_segment(
                [project([-1.0, value, 0.0]).0, project([1.0, value, 0.0]).0],
                grid,
            );
            painter.line_segment(
                [project([value, -1.0, 0.0]).0, project([value, 1.0, 0.0]).0],
                grid,
            );
        }
        let maximum = data.max_frequency().max(f32::EPSILON);
        let mut points = Vec::with_capacity(data.windows.len() * 256);
        for (index, window) in data.windows.iter().enumerate() {
            let y = if data.windows.len() == 1 {
                0.0
            } else {
                index as f32 / (data.windows.len() - 1) as f32 * 2.0 - 1.0
            };
            for (byte, &count) in window.counts.iter().enumerate() {
                if count == 0 {
                    continue;
                }
                let frequency = count as f32 / window.length as f32;
                let (position, depth) = project([
                    byte as f32 / 255.0 * 2.0 - 1.0,
                    y,
                    frequency / maximum * self.height,
                ]);
                if rect.contains(position) {
                    points.push((depth, position, index, byte, frequency));
                }
            }
        }
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        let hovered = response.hover_pos().and_then(|position| {
            points
                .iter()
                .rev()
                .filter_map(|p| {
                    let distance = p.1.distance_sq(position);
                    (distance <= 49.0).then_some((distance, p.2, p.3))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
        });
        let mut mesh = egui::epaint::Mesh::default();
        for &(_, position, index, byte, _) in &points {
            let selected = index == self.selected_row && byte == self.selected_byte;
            let radius = if selected {
                4.5
            } else {
                (1.8 * self.zoom.sqrt()).clamp(1.5, 3.5)
            };
            mesh.add_colored_rect(
                Rect::from_center_size(position, Vec2::splat(radius * 2.0)),
                if selected {
                    Color32::WHITE
                } else {
                    entropy_color(data.windows[index].entropy)
                },
            );
        }
        painter.add(egui::Shape::mesh(mesh));
        let label = |position: Pos2, text: String| {
            let width = (text.len() as f32 * 6.6).min(rect.width() - 16.0);
            let position = Pos2::new(
                position
                    .x
                    .clamp(rect.left() + 8.0, rect.right() - width - 8.0),
                position.y.clamp(rect.top() + 8.0, rect.bottom() - 18.0),
            );
            painter.text(
                position,
                egui::Align2::LEFT_TOP,
                text,
                egui::FontId::monospace(11.0),
                Color32::from_rgb(183, 203, 207),
            );
        };
        label(
            rect.left_top() + Vec2::splat(10.0),
            format!(
                "{} exact regions · {} bytes counted · nonzero cells shown",
                data.windows.len(),
                data.length
            ),
        );
        label(project([-1.0, -1.0, 0.0]).0, "byte 00".into());
        label(project([1.0, -1.0, 0.0]).0, "byte FF".into());
        label(
            project([-1.0, 1.0, 0.0]).0,
            format!(
                "source end +0x{:X}",
                data.windows.last().unwrap().offset + data.windows.last().unwrap().length as u64
            ),
        );
        let origin = project([-1.0, -1.0, 0.0]).0;
        let peak = project([-1.0, -1.0, self.height]).0;
        painter.line_segment([origin, peak], Stroke::new(1.5, ACCENT));
        label(peak, format!("{:.2}%", maximum * 100.0));
        if let Some((_, index, byte)) = hovered {
            response.clone().on_hover_ui(|ui| {
                let window = &data.windows[index];
                ui.monospace(format!(
                    "source+0x{:X}..0x{:X}\nbyte {byte:02X}: {} / {}\nentropy {:.4} bits/byte",
                    window.offset,
                    window.offset + window.length as u64,
                    window.counts[byte],
                    window.length,
                    window.entropy
                ));
            });
            if response.clicked() {
                self.selected_row = index;
                self.selected_byte = byte;
                ui.ctx().request_repaint();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_remains_finite_at_camera_bounds() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 300.0));
        for pitch in [0.15, 1.5] {
            for yaw in [-std::f32::consts::PI, std::f32::consts::PI] {
                for zoom in [0.5, 4.0] {
                    let (point, depth) = project(
                        [1.0, 1.0, 2.0],
                        yaw,
                        pitch,
                        rect,
                        zoom,
                        camera_bounds(yaw, pitch, 2.0),
                    );
                    assert!(point.x.is_finite() && point.y.is_finite() && depth.is_finite());
                }
            }
        }
    }

    #[test]
    fn default_zoom_fits_every_corner_at_camera_bounds() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 350.0));
        for pitch in [0.15, 0.8, 1.5] {
            for yaw in [-2.7, -0.65, 1.3] {
                for height in [0.3, 1.0, 2.0] {
                    let bounds = camera_bounds(yaw, pitch, height);
                    for x in [-1.0, 1.0] {
                        for y in [-1.0, 1.0] {
                            for z in [0.0, height] {
                                assert!(
                                    rect.shrink(20.0).contains(
                                        project([x, y, z], yaw, pitch, rect, 1.0, bounds).0
                                    )
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
