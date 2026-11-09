pub mod ingest;
pub mod layout;
pub mod mock;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Bfs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Personne,
    Organisation,
    Lieu,
    Evenement,
    Inconnu,
}

impl Kind {
    pub const MOCK: [Kind; 4] = [Kind::Personne, Kind::Organisation, Kind::Lieu, Kind::Evenement];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Personne => "Personne",
            Kind::Organisation => "Organisation",
            Kind::Lieu => "Lieu",
            Kind::Evenement => "Événement",
            Kind::Inconnu => "Inconnu",
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeData {
    pub label: String,
    pub kind: Kind,
}

/// Graphe orienté pondéré : les poids des arêtes servent de probabilités de transition.
pub type Graph = DiGraph<NodeData, f32>;

#[derive(Clone, Debug, Default)]
pub struct GraphStats {
    pub nodes: usize,
    pub edges: usize,
    pub density: f64,
    pub avg_degree: f64,
    pub max_out_degree: usize,
}

impl GraphStats {
    pub fn compute(g: &Graph) -> Self {
        let (n, e) = (g.node_count(), g.edge_count());
        GraphStats {
            nodes: n,
            edges: e,
            density: if n > 1 { e as f64 / (n as f64 * (n as f64 - 1.0)) } else { 0.0 },
            avg_degree: if n > 0 { e as f64 / n as f64 } else { 0.0 },
            max_out_degree: g.node_indices().map(|i| g.neighbors(i).count()).max().unwrap_or(0),
        }
    }
}

/// Graphe + positions de rendu + statistiques, partagé entre GUI et MCP.
pub struct Dataset {
    pub graph: Graph,
    pub pos: Vec<[f32; 2]>,
    pub stats: GraphStats,
    pub ingest_ms: u128,
}

impl Dataset {
    pub fn new(graph: Graph) -> Self {
        let pos = layout::compute(&graph);
        let stats = GraphStats::compute(&graph);
        Dataset { graph, pos, stats, ingest_ms: 0 }
    }
}

/// Nombre de nœuds atteignables depuis `start` (parcours en largeur).
pub fn reachable_count(g: &Graph, start: NodeIndex) -> usize {
    let mut bfs = Bfs::new(g, start);
    let mut n = 0;
    while bfs.next(g).is_some() {
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn mock_is_deterministic() {
        let a = mock::generate(500, 1_500, 7);
        let b = mock::generate(500, 1_500, 7);
        assert_eq!(a.node_count(), b.node_count());
        assert_eq!(a.edge_count(), b.edge_count());
    }

    #[test]
    fn stats_on_small_graph() {
        let mut g = Graph::new();
        let n: Vec<_> = (0..3)
            .map(|i| g.add_node(NodeData { label: format!("n{i}"), kind: Kind::Inconnu }))
            .collect();
        g.add_edge(n[0], n[1], 1.0);
        g.add_edge(n[0], n[2], 1.0);
        let s = GraphStats::compute(&g);
        assert_eq!((s.nodes, s.edges, s.max_out_degree), (3, 2, 2));
        assert_eq!(reachable_count(&g, n[0]), 3);
    }

    #[test]
    fn stress_ingest_and_traverse_100k() {
        let t = Instant::now();
        let g = mock::generate(100_000, 400_000, 1);
        let ingest = t.elapsed();
        assert_eq!(g.node_count(), 100_000);
        assert!(g.edge_count() > 399_000);

        let t = Instant::now();
        let reach = reachable_count(&g, NodeIndex::new(0));
        let bfs = t.elapsed();

        let t = Instant::now();
        let total: usize = g.node_indices().map(|i| g.neighbors(i).count()).sum();
        let scan = t.elapsed();

        println!("ingestion 100k/400k : {ingest:?}");
        println!("BFS ({reach} nœuds)   : {bfs:?}");
        println!("scan degrés ({total}) : {scan:?}");
        assert!(reach > 1);
        assert!(ingest.as_secs() < 30 && bfs.as_secs() < 30);
    }

    #[test]
    fn stress_full_dataset_100k() {
        let t = Instant::now();
        let ds = Dataset::new(mock::generate(100_000, 400_000, 2));
        println!("Dataset complet (layout + stats) : {:?}", t.elapsed());
        assert_eq!(ds.pos.len(), 100_000);
    }
}