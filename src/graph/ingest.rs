use super::{Graph, Kind, NodeData};
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

/// Parse une liste d'arêtes `source,cible[,poids]` (séparateurs `,` `;` ou tabulation).
/// Les lignes vides, commentées (`#`) ou d'en-tête sont ignorées.
pub fn from_edge_list(text: &str) -> Graph {
    let mut g = Graph::new();
    let mut ids: HashMap<&str, NodeIndex> = HashMap::new();
    for (lineno, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split(|c| c == ',' || c == ';' || c == '\t').map(str::trim);
        let (Some(a), Some(b)) = (it.next(), it.next()) else { continue };
        let mut w = 1.0f32;
        if let Some(s) = it.next() {
            match s.parse::<f32>() {
                Ok(v) => w = v.max(0.0),
                Err(_) if lineno == 0 => continue,
                Err(_) => {}
            }
        }
        let node = |name: &'static str, label: &str, g: &mut Graph| {
            let _ = name;
            g.add_node(NodeData { label: label.to_string(), kind: Kind::Inconnu })
        };
        let ia = *ids.entry(a).or_insert_with(|| node("a", a, &mut g));
        let ib = *ids.entry(b).or_insert_with(|| node("b", b, &mut g));
        g.add_edge(ia, ib, w);
    }
    g
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_edge_list() {
        let g = super::from_edge_list("source,cible,poids\na,b,0.5\nb,c\n# note\n\na;c;2");
        assert_eq!((g.node_count(), g.edge_count()), (3, 3));
    }
}