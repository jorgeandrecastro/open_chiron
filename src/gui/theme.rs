use crate::graph::Kind;
use eframe::egui::{self, vec2, Color32, FontId, Rounding, Stroke, TextStyle};

pub const ACCENT: Color32 = Color32::from_rgb(10, 132, 255);

pub fn kind_color(k: Kind) -> Color32 {
    match k {
        Kind::Personne => Color32::from_rgb(10, 132, 255),
        Kind::Organisation => Color32::from_rgb(191, 90, 242),
        Kind::Lieu => Color32::from_rgb(48, 209, 88),
        Kind::Evenement => Color32::from_rgb(255, 159, 10),
        Kind::Inconnu => Color32::from_rgb(142, 142, 147),
    }
}

/// Thème inspiré d'Apple : fonds neutres, coins arrondis, traits discrets, accent bleu système.
pub fn apply(ctx: &egui::Context, dark: bool) {
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let (bg, idle, hover, active, line) = if dark {
        (
            Color32::from_rgb(28, 28, 30),
            Color32::from_rgb(58, 58, 60),
            Color32::from_rgb(72, 72, 74),
            Color32::from_rgb(88, 88, 90),
            Color32::from_rgba_unmultiplied(255, 255, 255, 24),
        )
    } else {
        (
            Color32::from_rgb(242, 242, 247),
            Color32::from_rgb(229, 229, 234),
            Color32::from_rgb(216, 216, 222),
            Color32::from_rgb(199, 199, 204),
            Color32::from_rgba_unmultiplied(0, 0, 0, 24),
        )
    };
    v.panel_fill = bg;
    v.window_fill = bg;
    v.window_rounding = Rounding::same(14.0);
    v.selection.bg_fill = ACCENT;
    v.hyperlink_color = ACCENT;

    let w = &mut v.widgets;
    for ws in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        ws.rounding = Rounding::same(10.0);
        ws.bg_stroke = Stroke::NONE;
    }
    w.noninteractive.rounding = Rounding::same(10.0);
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, line);
    w.inactive.bg_fill = idle;
    w.inactive.weak_bg_fill = idle;
    w.hovered.bg_fill = hover;
    w.hovered.weak_bg_fill = hover;
    w.active.bg_fill = active;
    w.active.weak_bg_fill = active;
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 10.0);
        s.spacing.button_padding = vec2(14.0, 7.0);
        s.text_styles.insert(TextStyle::Body, FontId::proportional(14.5));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(14.5));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
    });
}