use crate::graph::Graph;
use ndarray::{Array1, Array2};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Prediction {
    pub node: NodeIndex,
    pub probability: f64,
}

/// Chaîne de Markov restreinte au voisinage sortant (3 sauts, `max_nodes` max) de `origin`.
/// Les poids d'arêtes sont normalisés par ligne ; un nœud sans sortie devient absorbant.
/// L'indice 0 est toujours `origin`.
pub fn local_chain(g: &Graph, origin: NodeIndex, max_nodes: usize) -> (Vec<NodeIndex>, Array2<f64>) {
    let mut nodes = vec![origin];
    let mut idx: HashMap<NodeIndex, usize> = HashMap::from([(origin, 0)]);
    let mut frontier = vec![origin];
    for _ in 0..3 {
        let mut next = Vec::new();
        for &u in &frontier {
            for v in g.neighbors(u) {
                if nodes.len() >= max_nodes {
                    break;
                }
                if !idx.contains_key(&v) {
                    idx.insert(v, nodes.len());
                    nodes.push(v);
                    next.push(v);
                }
            }
        }
        frontier = next;
    }

    let n = nodes.len();
    let mut p = Array2::<f64>::zeros((n, n));
    for (i, &u) in nodes.iter().enumerate() {
        for e in g.edges(u) {
            if let Some(&j) = idx.get(&e.target()) {
                p[[i, j]] += *e.weight() as f64;
            }
        }
        let sum: f64 = p.row(i).sum();
        if sum > 0.0 {
            p.row_mut(i).mapv_inplace(|x| x / sum);
        } else {
            p[[i, i]] = 1.0;
        }
    }
    (nodes, p)
}

/// Distribution de probabilité après `steps` pas depuis `origin`, triée, tronquée à `top`.
pub fn predict(g: &Graph, origin: NodeIndex, steps: usize, top: usize) -> Vec<Prediction> {
    let (nodes, p) = local_chain(g, origin, 128);
    let mut pi = Array1::<f64>::zeros(nodes.len());
    pi[0] = 1.0;
    for _ in 0..steps {
        pi = pi.dot(&p);
    }
    let mut out: Vec<Prediction> = nodes
        .iter()
        .zip(pi.iter())
        .map(|(&node, &probability)| Prediction { node, probability })
        .collect();
    out.sort_by(|a, b| b.probability.total_cmp(&a.probability));
    out.truncate(top);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::mock;
    use std::time::Instant;

    #[test]
    fn distribution_sums_to_one() {
        let g = mock::generate(1_000, 4_000, 3);
        let all = predict(&g, NodeIndex::new(0), 4, usize::MAX);
        let sum: f64 = all.iter().map(|p| p.probability).sum();
        assert!((sum - 1.0).abs() < 1e-9, "somme = {sum}");
    }

    #[test]
    fn stress_predict_on_100k() {
        let g = mock::generate(100_000, 400_000, 1);
        let t = Instant::now();
        for i in 0..1_000 {
            predict(&g, NodeIndex::new(i * 97), 3, 8);
        }
        println!("1000 prédictions Markov sur 100k nœuds : {:?}", t.elapsed());
    }
}