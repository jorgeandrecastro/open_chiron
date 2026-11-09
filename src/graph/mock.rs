use super::{Graph, Kind, NodeData};
use petgraph::graph::NodeIndex;
use rand::{rngs::StdRng, Rng, SeedableRng};

/// Génère un graphe synthétique reproductible : 70 % de liens locaux, 30 % de liens aléatoires.
pub fn generate(nodes: usize, edges: usize, seed: u64) -> Graph {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut g = Graph::with_capacity(nodes, edges);
    for i in 0..nodes {
        g.add_node(NodeData {
            label: format!("Entité {i}"),
            kind: Kind::MOCK[rng.gen_range(0..4)],
        });
    }
    if nodes < 2 {
        return g;
    }
    for _ in 0..edges {
        let a = rng.gen_range(0..nodes);
        let b = if rng.gen_bool(0.7) {
            (a + rng.gen_range(1..=50)) % nodes
        } else {
            rng.gen_range(0..nodes)
        };
        if a != b {
            g.add_edge(NodeIndex::new(a), NodeIndex::new(b), rng.gen_range(0.1..1.0));
        }
    }
    g
}