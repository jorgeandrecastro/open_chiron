use super::Graph;

/// Placement en spirale de Fibonacci (O(n)) ; affiné par un layout à forces pour les petits graphes.
pub fn compute(g: &Graph) -> Vec<[f32; 2]> {
    let n = g.node_count();
    let mut pos: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            let r = 14.0 * (i as f32).sqrt();
            let a = i as f32 * 2.399_963;
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    if (2..=800).contains(&n) {
        force_directed(g, &mut pos);
    }
    pos
}

fn force_directed(g: &Graph, pos: &mut [[f32; 2]]) {
    const ITER: usize = 120;
    let (n, k) = (pos.len(), 40.0f32);
    for it in 0..ITER {
        let temp = 30.0 * (1.0 - it as f32 / ITER as f32) + 0.5;
        let mut disp = vec![[0.0f32; 2]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = (pos[i][0] - pos[j][0], pos[i][1] - pos[j][1]);
                let f = k * k / (dx * dx + dy * dy).max(0.01);
                disp[i][0] += dx * f;
                disp[i][1] += dy * f;
                disp[j][0] -= dx * f;
                disp[j][1] -= dy * f;
            }
        }
        for e in g.edge_indices() {
            let (a, b) = g.edge_endpoints(e).unwrap();
            let (a, b) = (a.index(), b.index());
            let (dx, dy) = (pos[a][0] - pos[b][0], pos[a][1] - pos[b][1]);
            let f = (dx * dx + dy * dy).sqrt().max(0.01) / k;
            disp[a][0] -= dx * f;
            disp[a][1] -= dy * f;
            disp[b][0] += dx * f;
            disp[b][1] += dy * f;
        }
        for i in 0..n {
            let d = (disp[i][0].powi(2) + disp[i][1].powi(2)).sqrt().max(0.01);
            let s = d.min(temp) / d;
            pos[i][0] += disp[i][0] * s;
            pos[i][1] += disp[i][1] * s;
        }
    }
}