mod canvas;
mod theme;

use crate::analytics::markov::{self, Prediction};
use crate::graph::{self, ingest, mock, Dataset};
use crate::mcp::SharedDataset;
use canvas::Viewport;
use eframe::egui::{self, vec2, Margin, RichText, Sense};
use petgraph::graph::NodeIndex;
use petgraph::Direction;
use std::sync::{mpsc, Arc};
use std::time::Instant;
use tokio::runtime::Runtime;

enum Msg {
    Loaded(Dataset),
    Failed(String),
}

pub struct ChironApp {
    rt: Runtime,
    shared: SharedDataset,
    ds: Arc<Dataset>,
    tx: mpsc::Sender<Msg>,
    rx: mpsc::Receiver<Msg>,
    dark: bool,
    busy: bool,
    status: String,
    mock_nodes: usize,
    mock_degree: usize,
    csv_path: String,
    vp: Viewport,
    selected: Option<NodeIndex>,
    steps: usize,
    pred_key: Option<(NodeIndex, usize)>,
    preds: Vec<Prediction>,
    reach: usize,
}

impl ChironApp {
    pub fn new(cc: &eframe::CreationContext<'_>, rt: Runtime, shared: SharedDataset) -> Self {
        theme::apply(&cc.egui_ctx, true);
        let ds = shared.read().unwrap().clone();
        let (tx, rx) = mpsc::channel();
        ChironApp {
            rt,
            shared,
            ds,
            tx,
            rx,
            dark: true,
            busy: false,
            status: String::new(),
            mock_nodes: 100_000,
            mock_degree: 4,
            csv_path: String::new(),
            vp: Viewport::default(),
            selected: None,
            steps: 1,
            pred_key: None,
            preds: Vec::new(),
            reach: 0,
        }
    }

    fn spawn_mock(&mut self, ctx: &egui::Context) {
        self.busy = true;
        self.status = "Génération…".into();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        let (n, deg) = (self.mock_nodes, self.mock_degree);
        let seed = Instant::now().elapsed().as_nanos() as u64 ^ 0x5eed;
        self.rt.spawn_blocking(move || {
            let t = Instant::now();
            let mut ds = Dataset::new(mock::generate(n, n * deg, seed));
            ds.ingest_ms = t.elapsed().as_millis();
            let _ = tx.send(Msg::Loaded(ds));
            ctx.request_repaint();
        });
    }

    fn spawn_csv(&mut self, ctx: &egui::Context) {
        self.busy = true;
        self.status = "Lecture du fichier…".into();
        let (tx, ctx, path) = (self.tx.clone(), ctx.clone(), self.csv_path.clone());
        self.rt.spawn(async move {
            let t = Instant::now();
            let msg = match tokio::fs::read_to_string(&path).await {
                Ok(text) => match tokio::task::spawn_blocking(move || Dataset::new(ingest::from_edge_list(&text))).await {
                    Ok(mut ds) => {
                        ds.ingest_ms = t.elapsed().as_millis();
                        Msg::Loaded(ds)
                    }
                    Err(e) => Msg::Failed(e.to_string()),
                },
                Err(e) => Msg::Failed(format!("{path}: {e}")),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    fn pump_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            self.busy = false;
            match msg {
                Msg::Loaded(ds) => {
                    let ds = Arc::new(ds);
                    *self.shared.write().unwrap() = ds.clone();
                    self.status = format!("{} nœuds, {} relations en {} ms", ds.stats.nodes, ds.stats.edges, ds.ingest_ms);
                    self.ds = ds;
                    self.selected = None;
                    self.pred_key = None;
                    self.vp.needs_fit = true;
                }
                Msg::Failed(e) => self.status = format!("Erreur : {e}"),
            }
        }
    }

    /// Recalcule les prédictions si la sélection ou l'horizon a changé ; renvoie `true` si c'est le cas.
    fn refresh_predictions(&mut self) -> bool {
        let key = self.selected.map(|n| (n, self.steps));
        if key == self.pred_key {
            return false;
        }
        self.pred_key = key;
        match key {
            Some((n, steps)) => {
                self.preds = markov::predict(&self.ds.graph, n, steps, 8);
                self.reach = graph::reachable_count(&self.ds.graph, n);
            }
            None => self.preds.clear(),
        }
        true
    }

    fn left_panel(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui.label(RichText::new("Open Chiron").size(24.0).strong());
        ui.label(RichText::new("Analyse de graphes souveraine").weak());
        ui.add_space(14.0);

        section(ui, "INGESTION");
        ui.label("Données synthétiques");
        ui.add(egui::Slider::new(&mut self.mock_nodes, 1_000..=500_000).logarithmic(true).text("nœuds"));
        ui.add(egui::Slider::new(&mut self.mock_degree, 1..=10).text("liens / nœud"));
        if ui.add_enabled(!self.busy, egui::Button::new("Générer")).clicked() {
            self.spawn_mock(&ctx);
        }
        ui.add_space(6.0);
        ui.label("Fichier d'arêtes (CSV)");
        ui.add(
            egui::TextEdit::singleline(&mut self.csv_path)
                .hint_text("source,cible,poids")
                .desired_width(f32::INFINITY),
        );
        if ui.add_enabled(!self.busy && !self.csv_path.is_empty(), egui::Button::new("Importer")).clicked() {
            self.spawn_csv(&ctx);
        }
        ui.horizontal(|ui| {
            if self.busy {
                ui.spinner();
            }
            ui.label(RichText::new(&self.status).weak().small());
        });

        ui.add_space(14.0);
        section(ui, "STATISTIQUES");
        let s = &self.ds.stats;
        egui::Grid::new("stats").num_columns(2).spacing([16.0, 8.0]).show(ui, |ui| {
            for (k, v) in [
                ("Nœuds", s.nodes.to_string()),
                ("Relations", s.edges.to_string()),
                ("Densité", format!("{:.2e}", s.density)),
                ("Degré moyen", format!("{:.2}", s.avg_degree)),
                ("Degré sortant max", s.max_out_degree.to_string()),
                ("Dernière ingestion", format!("{} ms", self.ds.ingest_ms)),
            ] {
                ui.label(RichText::new(k).weak());
                ui.label(v);
                ui.end_row();
            }
        });

        ui.add_space(14.0);
        section(ui, "APPARENCE");
        ui.horizontal(|ui| {
            let (a, b) = (
                ui.selectable_label(!self.dark, "Clair").clicked(),
                ui.selectable_label(self.dark, "Sombre").clicked(),
            );
            if a || b {
                self.dark = b;
                theme::apply(&ctx, self.dark);
            }
        });
    }

    fn right_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Propriétés").size(18.0).strong());
        let Some(sel) = self.selected else {
            ui.label(RichText::new("Sélectionnez un nœud dans le graphe.").weak());
            return;
        };
        let g = &self.ds.graph;
        let n = &g[sel];
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 5.0, theme::kind_color(n.kind));
            ui.label(RichText::new(&n.label).strong());
        });
        egui::Grid::new("node").num_columns(2).spacing([16.0, 8.0]).show(ui, |ui| {
            for (k, v) in [
                ("ID", sel.index().to_string()),
                ("Type", n.kind.label().to_string()),
                ("Degré sortant", g.neighbors(sel).count().to_string()),
                ("Degré entrant", g.neighbors_directed(sel, Direction::Incoming).count().to_string()),
                ("Atteignables", self.reach.to_string()),
            ] {
                ui.label(RichText::new(k).weak());
                ui.label(v);
                ui.end_row();
            }
        });

        ui.add_space(14.0);
        section(ui, "PRÉDICTION MARKOV");
        ui.add(egui::Slider::new(&mut self.steps, 1..=6).text("pas"));
        for p in &self.preds {
            ui.label(&g[p.node].label);
            ui.add(
                egui::ProgressBar::new(p.probability as f32)
                    .text(format!("{:.1} %", p.probability * 100.0))
                    .desired_width(ui.available_width()),
            );
        }
    }
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(11.0).strong().weak());
}

impl eframe::App for ChironApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump_messages();

        let side = egui::Frame::side_top_panel(&ctx.style()).inner_margin(Margin::same(16.0));
        egui::SidePanel::left("gauche")
            .exact_width(290.0)
            .resizable(false)
            .frame(side)
            .show(ctx, |ui| self.left_panel(ui));
        egui::SidePanel::right("droite")
            .exact_width(290.0)
            .resizable(false)
            .frame(side)
            .show(ctx, |ui| self.right_panel(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(Margin::same(12.0)))
            .show(ctx, |ui| canvas::show(ui, &mut self.vp, &self.ds, &mut self.selected));

        if self.refresh_predictions() {
            ctx.request_repaint();
        }
    }
}