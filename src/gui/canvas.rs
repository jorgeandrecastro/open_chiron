use super::theme::{kind_color, ACCENT};
use crate::graph::Dataset;
use eframe::egui::{self, vec2, Align2, Color32, FontId, Pos2, Rounding, Sense, Stroke, Vec2};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

const MAX_EDGES: usize = 6_000;
const MAX_NODES: usize = 25_000;

/// Caméra 2D : `pan` est le décalage en pixels de l'origine monde par rapport au centre de la zone.
pub struct Viewport {
    pub zoom: f32,
    pub pan: Vec2,
    pub needs_fit: bool,
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport { zoom: 1.0, pan: Vec2::ZERO, needs_fit: true }
    }
}

impl Viewport {
    fn project(&self, center: Pos2, p: [f32; 2]) -> Pos2 {
        center + self.pan + vec2(p[0], p[1]) * self.zoom
    }

    fn fit(&mut self, pos: &[[f32; 2]], size: Vec2) {
        let m = pos.iter().fold(1.0f32, |m, p| m.max(p[0].abs()).max(p[1].abs()));
        self.zoom = 0.45 * size.x.min(size.y) / m;
        self.pan = Vec2::ZERO;
        self.needs_fit = false;
    }
}

/// Canvas interactif : pan (glisser), zoom (molette, centré sur le curseur), sélection (clic).
/// Le rendu est limité (échantillonnage + culling) pour rester fluide sur 100 000+ nœuds.
pub fn show(ui: &mut egui::Ui, vp: &mut Viewport, ds: &Dataset, selected: &mut Option<NodeIndex>) {
    let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = resp.rect;
    let dark = ui.visuals().dark_mode;
    let bg = if dark { Color32::from_rgb(18, 18, 20) } else { Color32::WHITE };
    painter.rect_filled(rect, Rounding::same(14.0), bg);
    if vp.needs_fit {
        vp.fit(&ds.pos, rect.size());
    }
    let c = rect.center();

    if resp.dragged() {
        vp.pan += resp.drag_delta();
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if let (true, Some(m)) = (scroll != 0.0, resp.hover_pos()) {
            let anchor = (m - c - vp.pan) / vp.zoom;
            vp.zoom = (vp.zoom * (scroll * 0.005).exp()).clamp(1e-3, 300.0);
            vp.pan = m - c - anchor * vp.zoom;
        }
    }

    let g = &ds.graph;
    let view = rect.expand(20.0);

    let edge_col = if dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, 22)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 30)
    };
    let stride = (g.edge_count() / MAX_EDGES).max(1);
    for e in g.edge_references().step_by(stride) {
        let a = vp.project(c, ds.pos[e.source().index()]);
        let b = vp.project(c, ds.pos[e.target().index()]);
        if view.contains(a) || view.contains(b) {
            painter.line_segment([a, b], Stroke::new(0.6_f32, edge_col));
        }
    }

    let visible: Vec<usize> = (0..ds.pos.len())
        .filter(|&i| view.contains(vp.project(c, ds.pos[i])))
        .collect();
    let r = (1.0 + vp.zoom.sqrt()).clamp(1.2, 7.0);
    for &i in visible.iter().step_by((visible.len() / MAX_NODES).max(1)) {
        let p = vp.project(c, ds.pos[i]);
        painter.circle_filled(p, r, kind_color(g[NodeIndex::new(i)].kind));
    }

    if vp.zoom > 2.0 && visible.len() <= 150 {
        let col = if dark { Color32::from_gray(200) } else { Color32::from_gray(60) };
        for &i in &visible {
            let p = vp.project(c, ds.pos[i]);
            painter.text(p + vec2(7.0, -7.0), Align2::LEFT_BOTTOM, &g[NodeIndex::new(i)].label, FontId::proportional(11.0), col);
        }
    }

    if let Some(sel) = *selected {
        let sp = vp.project(c, ds.pos[sel.index()]);
        for e in g.edges(sel) {
            let tp = vp.project(c, ds.pos[e.target().index()]);
            painter.line_segment([sp, tp], Stroke::new(1.2_f32, ACCENT));
            painter.circle_stroke(tp, r + 2.0, Stroke::new(1.0_f32, ACCENT));
        }
        painter.circle_stroke(sp, r + 4.0, Stroke::new(2.0_f32, ACCENT));
    }

    if resp.clicked() {
        if let Some(m) = resp.interact_pointer_pos() {
            *selected = visible
                .iter()
                .map(|&i| (i, (vp.project(c, ds.pos[i]) - m).length_sq()))
                .filter(|&(_, d)| d < 144.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| NodeIndex::new(i));
        }
    }
}