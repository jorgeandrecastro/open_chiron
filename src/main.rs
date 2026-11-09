mod analytics;
mod graph;
mod gui;
mod mcp;

use eframe::egui;
use graph::{mock, Dataset};
use std::sync::{Arc, RwLock};

fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime tokio");

    let initial = Arc::new(Dataset::new(mock::generate(600, 1_800, 42)));
    let shared: mcp::SharedDataset = Arc::new(RwLock::new(initial));

    let addr = std::env::var("CHIRON_MCP_ADDR").unwrap_or_else(|_| "127.0.0.1:7878".into());
    let state = shared.clone();
    rt.spawn(async move {
        if let Err(e) = mcp::serve(&addr, state).await {
            eprintln!("MCP indisponible: {e}");
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Open Chiron")
            .with_inner_size([1360.0, 840.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Open Chiron",
        options,
        Box::new(move |cc| Ok(Box::new(gui::ChironApp::new(cc, rt, shared)))),
    )
}