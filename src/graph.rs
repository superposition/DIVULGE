use divulge::domain::{Investigation, RecordData, RecordId, Relation};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use std::collections::{HashMap, HashSet};

pub const ACCENT: Color32 = Color32::from_rgb(111, 215, 191);
pub const TEXT: Color32 = Color32::from_rgb(226, 232, 237);
pub const MUTED: Color32 = Color32::from_rgb(150, 165, 179);
pub const BACKGROUND: Color32 = Color32::from_rgb(19, 24, 31);
pub const PANEL: Color32 = Color32::from_rgb(25, 31, 39);
const NODE_SIZE: Vec2 = Vec2::new(260.0, 110.0);
const MIN_ZOOM: f32 = 0.08;
const MAX_ZOOM: f32 = 1.6;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Narrative,
    Relationships,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    All,
    Branch,
    Neighbors,
}
#[derive(Clone, Copy)]
enum Navigation {
    Center,
    Fit,
    In,
    Out,
    Left,
    Right,
    Up,
    Down,
    Pin,
    Previous,
    Next,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
}

pub struct GraphView {
    pub layout: Layout,
    pub bounds: Rect,
    positions: HashMap<RecordId, Pos2>,
    pinned: HashSet<RecordId>,
    scope: Scope,
    anchor: RecordId,
    displayed: Vec<RecordId>,
    viewport: Vec2,
}
#[derive(Default)]
pub struct GraphResponse {
    pub selected: Option<usize>,
    pub moved: bool,
    pub focus_question: bool,
}
impl Default for GraphView {
    fn default() -> Self {
        Self {
            layout: Layout::Narrative,
            bounds: Rect::ZERO,
            positions: HashMap::new(),
            pinned: HashSet::new(),
            scope: Scope::All,
            anchor: RecordId(1),
            displayed: vec![],
            viewport: Vec2::new(600.0, 400.0),
        }
    }
}
pub fn kind_color(data: &RecordData) -> Color32 {
    match data {
        RecordData::Question => Color32::from_rgb(138, 178, 239),
        RecordData::Probe { .. } | RecordData::Review { .. } => Color32::from_rgb(174, 158, 220),
        RecordData::Observation(_)
        | RecordData::Firmware(_)
        | RecordData::Disassembly(_)
        | RecordData::Rea(_) => ACCENT,
        RecordData::Explanation { .. } | RecordData::TypeHypothesis(_) => {
            Color32::from_rgb(218, 197, 133)
        }
        RecordData::Unknown { .. } => Color32::from_rgb(225, 163, 117),
    }
}
fn relation_purpose(relation: Relation) -> &'static str {
    match relation {
        Relation::Investigates => "asks for a distinguishing check",
        Relation::Produces => "records the result of this action",
        Relation::Supports => "cites evidence for an interpretation",
        Relation::LeavesOpen => "retains an unanswered question",
        Relation::Suggests => "proposes a possible next inquiry",
        Relation::Contradicts => "records counter-evidence",
        Relation::Revises => "adds a correction; the original remains",
        Relation::Resolves => "records a scoped analyst resolution",
        Relation::Steers => "records a change in investigation direction",
        Relation::Reuses => "reuses previously checked evidence",
    }
}
fn coverage(session: &Investigation, id: RecordId) -> String {
    match session.record(id).map(|r| &r.data) {
        Some(RecordData::Disassembly(run)) => session
            .disassemblies
            .iter()
            .find(|r| r.id == *run)
            .map(|r| {
                format!(
                    "{:?} byte coverage / {} instructions",
                    r.coverage,
                    r.instructions.len()
                )
            })
            .unwrap_or_else(|| "Unavailable instruction coverage".into()),
        Some(RecordData::Observation(o)) => {
            format!("{} recorded bytes; meaning not established", o.bytes.len())
        }
        Some(RecordData::Unknown { .. }) => "Unresolved / no destination or answer inferred".into(),
        Some(RecordData::Explanation { basis, evidence }) => {
            format!("{basis} / {} cited observations", evidence.len())
        }
        Some(RecordData::TypeHypothesis(h)) => format!(
            "Hypothesis / {} supporting, {} counter-evidence",
            h.evidence.len(),
            h.counter_evidence.len()
        ),
        Some(RecordData::Rea(_)) => {
            "REA provider coverage and limitations in Harness Insight".into()
        }
        _ => "Relationship records intent or provenance; it does not prove runtime behavior".into(),
    }
}
impl GraphView {
    /// Presentation coordinates use canonical IDs, independently of record order or layout.
    pub fn append_records(&mut self, session: &Investigation) {
        for record in &session.records {
            if self.positions.contains_key(&record.id) {
                continue;
            }
            let parent = session
                .edges
                .iter()
                .rev()
                .find(|e| e.to == record.id)
                .and_then(|e| self.positions.get(&e.from))
                .copied();
            let mut position = parent
                .map(|p| p + Vec2::new(310.0, 0.0))
                .unwrap_or(Pos2::new(0.0, self.positions.len() as f32 * 154.0));
            // A finite search; live arrivals never move previous nodes or the camera.
            for _ in 0..=session.records.len() {
                let rect = Rect::from_min_size(position, NODE_SIZE).expand(14.0);
                if !self
                    .positions
                    .values()
                    .any(|p| rect.intersects(Rect::from_min_size(*p, NODE_SIZE)))
                {
                    break;
                }
                position.y += 154.0;
            }
            self.positions.insert(record.id, position);
        }
    }
    pub fn reset(&mut self, session: &Investigation) {
        self.positions.clear();
        self.pinned.clear();
        self.scope = Scope::All;
        self.displayed.clear();
        self.append_records(session);
        self.relayout(session);
        self.bounds = Rect::ZERO;
    }
    fn members(&self, session: &Investigation, visible: usize) -> Vec<RecordId> {
        let question = divulge::memory::branch_question(session, self.anchor);
        session
            .records
            .iter()
            .take(visible)
            .filter(|r| match self.scope {
                Scope::All => true,
                Scope::Branch => {
                    r.id == self.anchor
                        || question.is_some()
                            && divulge::memory::branch_question(session, r.id) == question
                }
                Scope::Neighbors => {
                    r.id == self.anchor
                        || session.edges.iter().any(|e| {
                            (e.from == self.anchor && e.to == r.id)
                                || (e.to == self.anchor && e.from == r.id)
                        })
                }
            })
            .map(|r| r.id)
            .collect()
    }
    fn relayout(&mut self, session: &Investigation) {
        self.append_records(session);
        // Narrative columns are explicit question branches, ordered by recorded chronology.
        let mut branches: Vec<(Option<RecordId>, Vec<RecordId>)> = vec![];
        for r in &session.records {
            let owner = divulge::memory::branch_question(session, r.id).map(|q| q.record());
            if let Some((_, records)) = branches.iter_mut().find(|(id, _)| *id == owner) {
                records.push(r.id);
            } else {
                branches.push((owner, vec![r.id]));
            }
        }
        let mut column = 0;
        for (_, ids) in branches {
            for (i, id) in ids.iter().enumerate() {
                if !self.pinned.contains(id) {
                    self.positions.insert(
                        *id,
                        Pos2::new((column + i / 12) as f32 * 330.0, (i % 12) as f32 * 160.0),
                    );
                }
            }
            column += ids.len().div_ceil(12);
        }
        if self.layout == Layout::Relationships {
            self.relax(session);
        }
        self.separate(session);
    }
    /// Deterministic, bounded and settled before reading; never runs each animation frame.
    fn relax(&mut self, session: &Investigation) {
        let ids: Vec<_> = session.records.iter().map(|r| r.id).collect();
        let columns = (ids.len() as f32).sqrt().ceil() as usize;
        for (i, id) in ids.iter().enumerate() {
            if !self.pinned.contains(id) {
                self.positions.insert(
                    *id,
                    Pos2::new((i % columns) as f32 * 330.0, (i / columns) as f32 * 170.0),
                );
            }
        }
        let indexes: HashMap<_, _> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        for step in 0..160 {
            let mut forces = vec![Vec2::ZERO; ids.len()];
            for i in 0..ids.len() {
                for j in i + 1..ids.len() {
                    let delta = self.positions[&ids[i]] - self.positions[&ids[j]];
                    let distance = delta.length().max(1.0);
                    let mut force = delta / distance * (45_000.0 / distance.powi(2));
                    let overlap = NODE_SIZE + Vec2::splat(40.0) - delta.abs();
                    if overlap.x > 0.0 && overlap.y > 0.0 {
                        force += if overlap.x < overlap.y {
                            Vec2::new(if delta.x < 0.0 { -overlap.x } else { overlap.x }, 0.0)
                        } else {
                            Vec2::new(0.0, if delta.y < 0.0 { -overlap.y } else { overlap.y })
                        } * 0.35;
                    }
                    forces[i] += force;
                    forces[j] -= force;
                }
            }
            for edge in &session.edges {
                let (Some(&i), Some(&j)) = (indexes.get(&edge.from), indexes.get(&edge.to)) else {
                    continue;
                };
                let delta = self.positions[&ids[j]] - self.positions[&ids[i]];
                let distance = delta.length().max(1.0);
                let force = delta / distance * (distance - 350.0) * 0.018;
                forces[i] += force;
                forces[j] -= force;
            }
            for (id, force) in ids.iter().zip(forces) {
                if !self.pinned.contains(id) {
                    *self.positions.get_mut(id).unwrap() +=
                        force * (12.0 / force.length().max(12.0)) * (1.0 - step as f32 / 200.0);
                }
            }
        }
    }
    fn separate(&mut self, session: &Investigation) {
        let ids: Vec<_> = session.records.iter().map(|r| r.id).collect();
        for _ in 0..80 {
            let mut changed = false;
            for i in 0..ids.len() {
                for j in i + 1..ids.len() {
                    let a = ids[i];
                    let b = ids[j];
                    if self.pinned.contains(&a) && self.pinned.contains(&b) {
                        continue;
                    }
                    let delta = self.positions[&b] - self.positions[&a];
                    let overlap = NODE_SIZE + Vec2::splat(24.0) - delta.abs();
                    if overlap.x <= 0.0 || overlap.y <= 0.0 {
                        continue;
                    }
                    let push = if overlap.x < overlap.y {
                        Vec2::new(
                            if delta.x < 0.0 {
                                -overlap.x - 0.1
                            } else {
                                overlap.x + 0.1
                            },
                            0.0,
                        )
                    } else {
                        Vec2::new(
                            0.0,
                            if delta.y < 0.0 {
                                -overlap.y - 0.1
                            } else {
                                overlap.y + 0.1
                            },
                        )
                    };
                    let pinned_a = self.pinned.contains(&a);
                    let pinned_b = self.pinned.contains(&b);
                    if !pinned_a {
                        *self.positions.get_mut(&a).unwrap() -=
                            push * if pinned_b { 1.0 } else { 0.5 };
                    }
                    if !pinned_b {
                        *self.positions.get_mut(&b).unwrap() +=
                            push * if pinned_a { 1.0 } else { 0.5 };
                    }
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }
    pub fn fit(&mut self) {
        let rect = self
            .displayed
            .iter()
            .filter_map(|id| self.positions.get(id))
            .fold(Rect::NOTHING, |rect, p| {
                rect.union(Rect::from_min_size(*p, NODE_SIZE))
            });
        if rect.is_finite() {
            self.bounds = rect.expand(32.0);
        }
    }
    fn zoom(&self) -> f32 {
        if self.bounds.is_positive() {
            (self.viewport / self.bounds.size())
                .min_elem()
                .clamp(MIN_ZOOM, MAX_ZOOM)
        } else {
            1.0
        }
    }
    fn change_zoom(&mut self, factor: f32) {
        let zoom = (self.zoom() * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.bounds = Rect::from_center_size(self.bounds.center(), self.viewport / zoom);
    }
    fn center_id(&mut self, id: RecordId) {
        if let Some(position) = self.positions.get(&id) {
            self.bounds = Rect::from_center_size(*position + NODE_SIZE * 0.5, self.viewport);
        }
    }
    fn arrange_scope(&mut self, session: &Investigation, visible: usize, selected: RecordId) {
        self.displayed = self.members(session, visible);
        if self.scope != Scope::All {
            let anchor = self.positions[&selected];
            let neighbors: Vec<_> = self
                .displayed
                .iter()
                .copied()
                .filter(|id| *id != selected)
                .collect();
            for (i, id) in neighbors.iter().enumerate() {
                if self.pinned.contains(id) {
                    continue;
                }
                let offset = if neighbors.len() <= 2 {
                    Vec2::new(0.0, if i == 0 { -154.0 } else { 154.0 })
                } else {
                    let slots = [
                        (-1.0, -1.0),
                        (0.0, -1.0),
                        (1.0, -1.0),
                        (-1.0, 0.0),
                        (1.0, 0.0),
                        (-1.0, 1.0),
                        (0.0, 1.0),
                        (1.0, 1.0),
                    ];
                    let (x, y) = slots[i % slots.len()];
                    Vec2::new(
                        x * 320.0 * (1 + i / slots.len()) as f32,
                        y * 154.0 * (1 + i / slots.len()) as f32,
                    )
                };
                self.positions.insert(*id, anchor + offset);
            }
        }
        if self.displayed.len() <= 6 {
            self.fit();
        } else {
            self.center_id(selected);
        }
    }
    pub fn focus_result(&mut self, session: &Investigation, index: usize, visible: usize) {
        self.append_records(session);
        let id = session.records[index].id;
        if !self.members(session, visible).contains(&id) {
            self.anchor = id;
            self.scope = Scope::Neighbors;
        }
        self.displayed = self.members(session, visible);
        self.center_id(id);
    }
    pub fn focus_question(&mut self, session: &Investigation, index: usize, visible: usize) {
        self.append_records(session);
        self.anchor = session.records[index].id;
        self.scope = Scope::Branch;
        self.displayed = self.members(session, visible);
        self.center_id(self.anchor);
    }
    fn navigate(
        &mut self,
        action: Navigation,
        session: &Investigation,
        visible: usize,
        selected: RecordId,
    ) -> Option<usize> {
        let index = session
            .records
            .iter()
            .position(|r| r.id == selected)
            .unwrap_or(0);
        let distance = 64.0 / self.zoom();
        match action {
            Navigation::Center => self.focus_result(session, index, visible),
            Navigation::Fit => self.fit(),
            Navigation::In => self.change_zoom(1.25),
            Navigation::Out => self.change_zoom(0.8),
            Navigation::Left => self.bounds = self.bounds.translate(Vec2::new(-distance, 0.0)),
            Navigation::Right => self.bounds = self.bounds.translate(Vec2::new(distance, 0.0)),
            Navigation::Up => self.bounds = self.bounds.translate(Vec2::new(0.0, -distance)),
            Navigation::Down => self.bounds = self.bounds.translate(Vec2::new(0.0, distance)),
            Navigation::Pin => {
                if !self.pinned.remove(&selected) {
                    self.pinned.insert(selected);
                }
            }
            Navigation::Previous | Navigation::Next => {
                let next = if matches!(action, Navigation::Previous) {
                    index.saturating_sub(1)
                } else {
                    (index + 1).min(visible.saturating_sub(1))
                };
                self.focus_result(session, next, visible);
                return Some(next);
            }
            Navigation::MoveLeft
            | Navigation::MoveRight
            | Navigation::MoveUp
            | Navigation::MoveDown => {
                let delta = match action {
                    Navigation::MoveLeft => Vec2::new(-32.0, 0.0),
                    Navigation::MoveRight => Vec2::new(32.0, 0.0),
                    Navigation::MoveUp => Vec2::new(0.0, -32.0),
                    _ => Vec2::new(0.0, 32.0),
                };
                if let Some(p) = self.positions.get_mut(&selected) {
                    *p += delta;
                    self.pinned.insert(selected);
                }
            }
        }
        None
    }
    fn controls(
        &mut self,
        ui: &mut egui::Ui,
        session: &Investigation,
        visible: usize,
        selected: RecordId,
    ) -> GraphResponse {
        let mut response = GraphResponse::default();
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            let previous = self.layout;
            ui.selectable_value(&mut self.layout, Layout::Narrative, "Narrative");
            ui.selectable_value(&mut self.layout, Layout::Relationships, "Relationships");
            if previous != self.layout {
                self.relayout(session);
                self.arrange_scope(session, visible, selected);
                response.moved = true;
            }
            ui.separator();
            for (scope, label) in [
                (Scope::All, "All"),
                (Scope::Branch, "Branch"),
                (Scope::Neighbors, "Neighbors"),
            ] {
                if ui.selectable_label(self.scope == scope, label).clicked() {
                    self.scope = scope;
                    self.anchor = selected;
                    if scope == Scope::All {
                        self.relayout(session);
                    }
                    self.arrange_scope(session, visible, selected);
                    response.moved = true;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("Center selected").on_hover_text("Alt+C / readable scale").clicked() { action = Some(Navigation::Center); }
            if ui.button("Fit scope").on_hover_text("Alt+F / overview; center a record to read it").clicked() { action = Some(Navigation::Fit); }
            if ui.button("-").on_hover_text("Zoom out / Alt+-").clicked() { action = Some(Navigation::Out); }
            if ui.button("+").on_hover_text("Zoom in / Alt+=").clicked() { action = Some(Navigation::In); }
            if ui.selectable_label(self.pinned.contains(&selected), "Pin selected").on_hover_text("Alt+P / retain this position during layout changes").clicked() { action = Some(Navigation::Pin); }
            ui.menu_button("Navigate",|ui| {
                for (nav,label) in [
                    (Navigation::Previous,"Previous record    Alt+PageUp"),(Navigation::Next,"Next record    Alt+PageDown"),
                    (Navigation::Left,"Pan left    Alt+Left"),(Navigation::Right,"Pan right    Alt+Right"),
                    (Navigation::Up,"Pan up    Alt+Up"),(Navigation::Down,"Pan down    Alt+Down"),
                    (Navigation::MoveLeft,"Move selected left    Alt+Shift+Left"),(Navigation::MoveRight,"Move selected right    Alt+Shift+Right"),
                    (Navigation::MoveUp,"Move selected up    Alt+Shift+Up"),(Navigation::MoveDown,"Move selected down    Alt+Shift+Down"),
                ] { if ui.button(label).clicked() { action = Some(nav); ui.close(); } }
                ui.label(format!("Tab / Shift+Tab and Enter operate controls and the record tree. {}+F finds a record.", crate::app::command_label()));
            });
        });
        for (key, nav) in [
            (egui::Key::C, Navigation::Center),
            (egui::Key::F, Navigation::Fit),
            (egui::Key::P, Navigation::Pin),
            (egui::Key::Equals, Navigation::In),
            (egui::Key::Minus, Navigation::Out),
            (egui::Key::PageUp, Navigation::Previous),
            (egui::Key::PageDown, Navigation::Next),
        ] {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, key)) {
                action = Some(nav);
            }
        }
        for (key, pan, movement) in [
            (egui::Key::ArrowLeft, Navigation::Left, Navigation::MoveLeft),
            (
                egui::Key::ArrowRight,
                Navigation::Right,
                Navigation::MoveRight,
            ),
            (egui::Key::ArrowUp, Navigation::Up, Navigation::MoveUp),
            (egui::Key::ArrowDown, Navigation::Down, Navigation::MoveDown),
        ] {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT | egui::Modifiers::SHIFT, key)) {
                action = Some(movement);
            } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, key)) {
                action = Some(pan);
            }
        }
        if let Some(action) = action {
            response.selected = self.navigate(action, session, visible, selected);
            response.moved = true;
        }
        response
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        session: &Investigation,
        visible: usize,
        selected: RecordId,
    ) -> GraphResponse {
        self.append_records(session);
        self.displayed = self.members(session, visible);
        if !self.bounds.is_positive() {
            self.center_id(selected);
        }
        let mut result = self.controls(ui, session, visible, selected);
        let ids: HashSet<_> = self.displayed.iter().copied().collect();
        let available: HashSet<_> = session.records.iter().take(visible).map(|r| r.id).collect();
        let links = session
            .edges
            .iter()
            .filter(|e| ids.contains(&e.from) && ids.contains(&e.to))
            .count();
        let boundary = session
            .edges
            .iter()
            .filter(|e| {
                available.contains(&e.from)
                    && available.contains(&e.to)
                    && (ids.contains(&e.from) != ids.contains(&e.to))
            })
            .count();
        ui.label(
            egui::RichText::new(format!(
                "{} / {} records · {} links · {} outside scope",
                ids.len(),
                visible,
                links,
                boundary
            ))
            .small()
            .color(MUTED),
        );
        ui.collapsing("Selected relationships / accessible list",|ui| {
            ui.label(coverage(session,selected));
            egui::ScrollArea::vertical().id_salt("graph_links").max_height(130.0).show(ui,|ui| {
                let mut count = 0;
                for edge in session.edges.iter().filter(|e| e.from == selected || e.to == selected) {
                    let other = if edge.from == selected { edge.to } else { edge.from };
                    if !available.contains(&other) { continue; }
                    let Some((index,record)) = session.records.iter().enumerate().find(|(_,r)| r.id == other) else { continue; };
                    count += 1;
                    let direction = if edge.from == selected { "to" } else { "from" };
                    if ui.add(egui::Button::new(egui::RichText::new(format!("{} {direction} #{}: {}",edge.relation.label(),other.0,record.title)).color(ACCENT)).frame(false).wrap()).on_hover_text(relation_purpose(edge.relation)).clicked() {
                        result.selected = Some(index); self.focus_result(session,index,visible); result.moved = true;
                    }
                    ui.label(egui::RichText::new(relation_purpose(edge.relation)).small().color(MUTED));
                }
                if count == 0 { ui.label("No recorded relationships at this replay checkpoint. This is not evidence that none exist."); }
            });
        });
        self.viewport = ui.available_size_before_wrap().max(Vec2::splat(1.0));
        let zoom = self.zoom();
        if zoom < 0.65 {
            ui.label(
                egui::RichText::new("Overview shows record IDs. Center selected for full labels.")
                    .small()
                    .color(MUTED),
            );
        }
        let scene = egui::Scene::new().zoom_range(MIN_ZOOM..=MAX_ZOOM).show(ui,&mut self.bounds,|ui| {
            let painter = ui.painter().clone();
            let world = ui.clip_rect();
            for edge in &session.edges {
                if !ids.contains(&edge.from) || !ids.contains(&edge.to) { continue; }
                let a = Rect::from_min_size(self.positions[&edge.from],NODE_SIZE);
                let b = Rect::from_min_size(self.positions[&edge.to],NODE_SIZE);
                if !a.union(b).intersects(world) { continue; }
                let delta = b.center() - a.center();
                let (start,end) = if delta.x.abs() > delta.y.abs() {
                    if delta.x > 0.0 { (a.right_center(),b.left_center()) } else { (a.left_center(),b.right_center()) }
                } else if delta.y > 0.0 { (a.center_bottom(),b.center_top()) } else { (a.center_top(),b.center_bottom()) };
                let active = edge.from == selected || edge.to == selected;
                let color = if active { ACCENT.gamma_multiply(0.75) } else { Color32::from_rgb(76,91,106) };
                let stroke = Stroke::new(if active {1.8} else {1.0},color);
                painter.line_segment([start,end],stroke);
                let direction = (end-start).normalized(); let side = Vec2::new(-direction.y,direction.x);
                let base = end-direction*10.0;
                painter.line_segment([base+side*4.0,end],stroke); painter.line_segment([base-side*4.0,end],stroke);
                if active && zoom >= 0.65 {
                    let midpoint = start.lerp(end,0.5);
                    let label = painter.layout_no_wrap(edge.relation.label().into(),FontId::proportional(11.0),TEXT);
                    painter.rect_filled(Rect::from_center_size(midpoint,label.size()+Vec2::new(8.0,4.0)),3,BACKGROUND);
                    painter.galley(midpoint-label.size()*0.5,label,TEXT);
                }
            }
            for (index,record) in session.records.iter().take(visible).enumerate() {
                if !ids.contains(&record.id) { continue; }
                let rect = Rect::from_min_size(self.positions[&record.id],NODE_SIZE);
                if !rect.intersects(world) { continue; } // Viewport culling; tree remains the complete accessible index.
                let response = ui.interact(rect,ui.id().with(record.id.0),Sense::click_and_drag());
                let active = record.id == selected;
                response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button,ui.is_enabled(),active,
                    format!("Record {}. {}. {}. {}",record.id.0,record.data.label(),record.title,record.summary)));
                if response.clicked() { result.selected = Some(index); }
                if response.double_clicked() { result.selected = Some(index); result.focus_question = true; }
                if response.dragged() {
                    *self.positions.get_mut(&record.id).unwrap() += response.drag_delta();
                    self.pinned.insert(record.id); result.moved = true;
                }
                painter.rect(rect,6,if active {Color32::from_rgb(31,46,51)} else {PANEL},
                    Stroke::new(if active || response.has_focus() {2.0} else {1.0},if active || response.has_focus() {ACCENT} else {Color32::from_rgb(62,74,88)}),StrokeKind::Inside);
                if zoom < 0.65 {
                    painter.text(rect.center(),Align2::CENTER_CENTER,format!("#{}",record.id.0),FontId::monospace(11.0/zoom),TEXT);
                } else {
                    let color = kind_color(&record.data);
                    painter.text(rect.min+Vec2::new(12.0,17.0),Align2::LEFT_CENTER,record.data.label().to_uppercase(),FontId::proportional(11.0),color);
                    painter.text(rect.right_top()+Vec2::new(-12.0,17.0),Align2::RIGHT_CENTER,
                        format!("{}#{:02}",if self.pinned.contains(&record.id) {"PIN "} else {""},record.id.0),FontId::monospace(10.0),MUTED);
                    let mut title = egui::text::LayoutJob::simple(record.title.clone(),FontId::proportional(15.0),TEXT,NODE_SIZE.x-24.0);
                    title.wrap.max_rows = 2;
                    painter.galley(rect.min+Vec2::new(12.0,34.0),painter.layout_job(title),TEXT);
                    let mut summary = egui::text::LayoutJob::simple(record.summary.clone(),FontId::proportional(11.0),MUTED,NODE_SIZE.x-24.0);
                    summary.wrap.max_rows = 1;
                    painter.galley(rect.min+Vec2::new(12.0,84.0),painter.layout_job(summary),MUTED);
                }
                response.on_hover_ui(|ui| { ui.set_max_width(370.0); ui.strong(&record.title); ui.label(&record.summary); ui.label(coverage(session,record.id));
                    ui.label("Select for the full explanation. Drag to arrange and pin. Use Navigate for keyboard movement."); });
            }
        });
        result.moved |= scene.response.changed();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maximum_case_culls_offscreen_cards_at_readable_scale() {
        let mut session = Investigation::workshop();
        for id in 7..=200 {
            let mut record = session.records[2].clone();
            record.id = RecordId(id);
            record.question_id = Some(divulge::domain::QuestionId::new(5).unwrap());
            record.title = format!("Generated observation {id}");
            session.records.push(record);
        }
        session.validate().unwrap();
        let mut graph = GraphView::default();
        graph.reset(&session);
        graph.focus_result(&session, 100, 200);
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0))),
            ..Default::default()
        };
        let output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                graph.show(ui, &session, 200, RecordId(101));
            });
        });
        let rendered = output
            .shapes
            .iter()
            .filter(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => {
                    t.galley.job.text.starts_with("Generated observation")
                }
                _ => false,
            })
            .count();
        assert!(
            rendered > 0 && rendered < 30,
            "Rendered {rendered} generated cards"
        );
        assert_eq!(graph.positions.len(), 200);
        assert_eq!(graph.displayed.len(), 200);
    }
    #[test]
    fn egui_pointer_pan_works_at_both_workshop_sizes() {
        for size in [Vec2::new(1280.0, 720.0), Vec2::new(1920.0, 1080.0)] {
            let ctx = egui::Context::default();
            let mut session = Investigation::workshop();
            session.records[0].title =
                "A long question with enough detail to wrap safely. ".repeat(20);
            let mut graph = GraphView::default();
            graph.reset(&session);
            let frame = |graph: &mut GraphView, events| {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                    events,
                    ..Default::default()
                };
                let _ = ctx.run_ui(input, |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        graph.show(ui, &session, 6, RecordId(1));
                    });
                });
            };
            frame(&mut graph, vec![]);
            frame(&mut graph, vec![]);
            let start = Pos2::new(size.x - 60.0, size.y - 80.0);
            let end = start - Vec2::new(90.0, 45.0);
            let before = graph.bounds;
            frame(
                &mut graph,
                vec![
                    egui::Event::PointerMoved(start),
                    egui::Event::PointerButton {
                        pos: start,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            frame(&mut graph, vec![egui::Event::PointerMoved(end)]);
            frame(
                &mut graph,
                vec![egui::Event::PointerButton {
                    pos: end,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(
                (graph.bounds.center() - before.center()).length() > 20.0,
                "Pointer drag did not pan at {size:?}"
            );
        }
    }
    fn larger() -> Investigation {
        let mut session = Investigation::workshop();
        for _ in 0..40 {
            session
                .append_review(
                    divulge::history::ReviewKind::Steering,
                    RecordId(1),
                    "Compare the evidence".into(),
                    "Still provisional".into(),
                    vec![],
                )
                .unwrap();
        }
        session
    }
    #[test]
    fn live_append_preserves_arranged_nodes_and_camera() {
        let mut session = Investigation::workshop();
        let mut graph = GraphView::default();
        graph.reset(&session);
        graph.positions.insert(RecordId(3), Pos2::new(77.0, 81.0));
        graph.pinned.insert(RecordId(3));
        graph.focus_result(&session, 3, 6);
        graph.bounds = graph.bounds.translate(Vec2::new(180.0, -90.0));
        let before = graph.positions.clone();
        let bounds = graph.bounds;
        for _ in 0..5 {
            session
                .append_review(
                    divulge::history::ReviewKind::Steering,
                    RecordId(1),
                    "Compare".into(),
                    "Unknown".into(),
                    vec![],
                )
                .unwrap();
            graph.append_records(&session);
            for (id, position) in &before {
                assert_eq!(graph.positions[id], *position);
            }
            assert_eq!(graph.bounds, bounds);
        }
        assert_eq!(graph.positions.len(), session.records.len());
    }
    #[test]
    fn settled_layout_keeps_ids_pins_and_nonoverlapping_nodes() {
        let session = larger();
        let original = serde_json::to_value(&session).unwrap();
        let mut a = GraphView::default();
        a.reset(&session);
        a.positions.insert(RecordId(3), Pos2::new(-380.0, -210.0));
        a.pinned.insert(RecordId(3));
        a.layout = Layout::Relationships;
        let mut b = GraphView::default();
        b.reset(&session);
        b.positions.insert(RecordId(3), Pos2::new(-380.0, -210.0));
        b.pinned.insert(RecordId(3));
        b.layout = Layout::Relationships;
        a.relayout(&session);
        b.relayout(&session);
        assert_eq!(a.positions, b.positions);
        assert_eq!(a.positions[&RecordId(3)], Pos2::new(-380.0, -210.0));
        for (i, record) in session.records.iter().enumerate() {
            let rect = Rect::from_min_size(a.positions[&record.id], NODE_SIZE);
            for other in session.records.iter().skip(i + 1) {
                assert!(!rect.intersects(Rect::from_min_size(a.positions[&other.id], NODE_SIZE)));
            }
        }
        assert_eq!(serde_json::to_value(&session).unwrap(), original);
    }
    #[test]
    fn scopes_frontiers_and_zoom_are_bounded_without_inventing_records() {
        let session = larger();
        let mut graph = GraphView::default();
        graph.reset(&session);
        graph.scope = Scope::Neighbors;
        graph.anchor = RecordId(1);
        assert!(graph.members(&session, 3).iter().all(|id| id.0 <= 3));
        graph.scope = Scope::Branch;
        graph.anchor = RecordId(5);
        assert!(
            !graph
                .members(&session, session.records.len())
                .contains(&RecordId(3))
        );
        graph.focus_result(&session, 2, 6);
        assert!(graph.displayed.contains(&RecordId(3)));
        for _ in 0..50 {
            graph.change_zoom(2.0);
        }
        assert!((graph.zoom() - MAX_ZOOM).abs() < 0.001);
        for _ in 0..50 {
            graph.change_zoom(0.5);
        }
        assert!((graph.zoom() - MIN_ZOOM).abs() < 0.001);
        graph.focus_result(&session, 2, 6);
        assert!((graph.zoom() - 1.0).abs() < 0.001);
        let old = graph.bounds;
        graph.navigate(Navigation::Right, &session, 6, RecordId(3));
        assert!(graph.bounds.center().x > old.center().x);
    }
}
