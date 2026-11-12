use divulge::domain::{Investigation, RecordData, RecordId};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

pub const ACCENT: Color32 = Color32::from_rgb(111, 215, 191);
pub const TEXT: Color32 = Color32::from_rgb(226, 232, 237);
pub const MUTED: Color32 = Color32::from_rgb(150, 165, 179);
pub const BACKGROUND: Color32 = Color32::from_rgb(19, 24, 31);
pub const PANEL: Color32 = Color32::from_rgb(25, 31, 39);
const NODE_SIZE: Vec2 = Vec2::new(250.0, 98.0);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Narrative,
    Relationships,
}

pub struct GraphView {
    pub layout: Layout,
    pub bounds: Rect,
    positions: Vec<Pos2>,
}

pub struct GraphResponse {
    pub selected: Option<usize>,
    pub moved: bool,
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            layout: Layout::Narrative,
            bounds: Rect::ZERO,
            positions: Vec::new(),
        }
    }
}

pub fn kind_color(data: &RecordData) -> Color32 {
    match data {
        RecordData::Question => Color32::from_rgb(138, 178, 239),
        RecordData::Probe { .. } => Color32::from_rgb(174, 158, 220),
        RecordData::Observation(_) => ACCENT,
        RecordData::Explanation { .. } => Color32::from_rgb(218, 197, 133),
        RecordData::Unknown { .. } => Color32::from_rgb(225, 163, 117),
    }
}

impl GraphView {
    pub fn reset(&mut self, session: &Investigation) {
        self.positions = (0..session.records.len())
            .map(|i| match i {
                0..=3 => Pos2::new(0.0, i as f32 * 136.0),
                _ => Pos2::new(302.0, (i as f32 - 1.0) * 136.0),
            })
            .collect();
        if self.layout == Layout::Relationships {
            self.relax(session);
        }
        self.fit();
    }

    /// A deterministic, bounded force layout. It settles before reading starts.
    fn relax(&mut self, session: &Investigation) {
        let count = self.positions.len();
        if count > 200 {
            return;
        }
        for i in 0..count {
            let angle = i as f32 * std::f32::consts::TAU / count as f32;
            self.positions[i] = Pos2::new(angle.cos() * 340.0, angle.sin() * 340.0);
        }
        for step in 0..160 {
            let mut forces = vec![Vec2::ZERO; count];
            for i in 0..count {
                for j in (i + 1)..count {
                    let delta = self.positions[i] - self.positions[j];
                    let distance = delta.length().max(1.0);
                    let force = delta / distance * (34_000.0 / distance.powi(2));
                    forces[i] += force;
                    forces[j] -= force;
                }
            }
            for edge in &session.edges {
                let i = session
                    .records
                    .iter()
                    .position(|r| r.id == edge.from)
                    .unwrap();
                let j = session
                    .records
                    .iter()
                    .position(|r| r.id == edge.to)
                    .unwrap();
                let delta = self.positions[j] - self.positions[i];
                let distance = delta.length().max(1.0);
                let force = delta / distance * (distance - 330.0) * 0.028;
                forces[i] += force;
                forces[j] -= force;
            }
            let temperature = 1.0 - step as f32 / 200.0;
            for (position, force) in self.positions.iter_mut().zip(forces) {
                *position += force * (12.0 / force.length().max(12.0)) * temperature;
            }
        }
    }

    pub fn fit(&mut self) {
        self.bounds = self
            .positions
            .iter()
            .fold(Rect::NOTHING, |rect, pos| {
                rect.union(Rect::from_min_size(*pos, NODE_SIZE))
            })
            .expand(36.0);
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        session: &Investigation,
        visible: usize,
        selected: RecordId,
    ) -> GraphResponse {
        if self.positions.len() != session.records.len() {
            self.reset(session);
        }
        let mut selection = None;
        let mut node_moved = false;
        let scene = egui::Scene::new()
            .zoom_range(0.25..=1.6)
            .show(ui, &mut self.bounds, |ui| {
                let painter = ui.painter().clone();
                let world = ui.clip_rect();
                let grid = 28.0;
                let x0 = (world.left() / grid).floor() as i32;
                let y0 = (world.top() / grid).floor() as i32;
                // Bound background work even at very wide zoom levels.
                for x in x0..(x0 + (world.width() / grid).ceil().min(180.0) as i32 + 1) {
                    for y in y0..(y0 + (world.height() / grid).ceil().min(180.0) as i32 + 1) {
                        painter.circle_filled(
                            Pos2::new(x as f32 * grid, y as f32 * grid),
                            0.8,
                            Color32::from_rgb(40, 49, 60),
                        );
                    }
                }
                for edge in &session.edges {
                    let from = session
                        .records
                        .iter()
                        .position(|r| r.id == edge.from)
                        .unwrap();
                    let to = session
                        .records
                        .iter()
                        .position(|r| r.id == edge.to)
                        .unwrap();
                    if from >= visible || to >= visible {
                        continue;
                    }
                    let a = Rect::from_min_size(self.positions[from], NODE_SIZE);
                    let b = Rect::from_min_size(self.positions[to], NODE_SIZE);
                    let delta = b.center() - a.center();
                    let (start, end) = if delta.x.abs() > delta.y.abs() {
                        if delta.x > 0.0 {
                            (a.right_center(), b.left_center())
                        } else {
                            (a.left_center(), b.right_center())
                        }
                    } else if delta.y > 0.0 {
                        (a.center_bottom(), b.center_top())
                    } else {
                        (a.center_top(), b.center_bottom())
                    };
                    let highlighted = edge.from == selected || edge.to == selected;
                    let color = if highlighted {
                        ACCENT.gamma_multiply(0.75)
                    } else {
                        Color32::from_rgb(76, 91, 106)
                    };
                    let stroke = Stroke::new(if highlighted { 1.8 } else { 1.2 }, color);
                    painter.line_segment([start, end], stroke);
                    let direction = (end - start).normalized();
                    let perpendicular = Vec2::new(-direction.y, direction.x);
                    let tip_base = end - direction * 9.0;
                    painter.line_segment([tip_base + perpendicular * 4.0, end], stroke);
                    painter.line_segment([tip_base - perpendicular * 4.0, end], stroke);
                    if self.layout == Layout::Narrative && delta.x.abs() < 10.0 {
                        painter.text(
                            start.lerp(end, 0.5) + Vec2::new(10.0, 0.0),
                            Align2::LEFT_CENTER,
                            edge.relation.label(),
                            FontId::proportional(10.0),
                            MUTED,
                        );
                    }
                }
                for (i, record) in session.records.iter().take(visible).enumerate() {
                    let rect = Rect::from_min_size(self.positions[i], NODE_SIZE);
                    ui.expand_to_include_rect(rect);
                    let response =
                        ui.interact(rect, ui.id().with(record.id.0), Sense::click_and_drag());
                    if response.clicked() {
                        selection = Some(i);
                    }
                    if response.dragged() {
                        self.positions[i] += response.drag_delta();
                        node_moved = true;
                    }
                    let color = kind_color(&record.data);
                    let active = record.id == selected;
                    let fill = if active {
                        Color32::from_rgb(31, 46, 51)
                    } else if response.hovered() {
                        Color32::from_rgb(35, 43, 54)
                    } else {
                        PANEL
                    };
                    painter.rect(
                        rect,
                        8,
                        fill,
                        Stroke::new(
                            if active { 1.6 } else { 1.0 },
                            if active {
                                ACCENT
                            } else {
                                Color32::from_rgb(62, 74, 88)
                            },
                        ),
                        StrokeKind::Inside,
                    );
                    painter.circle_filled(rect.min + Vec2::new(17.0, 19.0), 3.0, color);
                    painter.text(
                        rect.min + Vec2::new(28.0, 19.0),
                        Align2::LEFT_CENTER,
                        record.data.label().to_uppercase(),
                        FontId::proportional(10.0),
                        color,
                    );
                    painter.text(
                        rect.right_top() + Vec2::new(-14.0, 19.0),
                        Align2::RIGHT_CENTER,
                        format!("{:02}", record.id.0),
                        FontId::monospace(10.0),
                        MUTED,
                    );
                    let title = painter.layout(
                        record.title.clone(),
                        FontId::proportional(15.0),
                        TEXT,
                        NODE_SIZE.x - 28.0,
                    );
                    painter.galley(rect.min + Vec2::new(14.0, 37.0), title, TEXT);
                    let summary = painter.layout(
                        record.summary.clone(),
                        FontId::proportional(11.0),
                        MUTED,
                        NODE_SIZE.x - 28.0,
                    );
                    painter.galley(rect.min + Vec2::new(14.0, 64.0), summary, MUTED);
                    response.on_hover_text(format!(
                        "{}\n{}",
                        record.data.rust_type(),
                        record.summary
                    ));
                }
            });
        GraphResponse {
            selected: selection,
            moved: node_moved || scene.response.changed(),
        }
    }
}
