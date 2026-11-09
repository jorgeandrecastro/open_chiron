//! Serveur JSON-RPC 2.0 local (TCP, un message JSON par ligne) exposant l'état du graphe
//! avec les méthodes MCP `initialize`, `tools/list` et `tools/call`.

use crate::analytics::markov;
use crate::graph::{reachable_count, Dataset, Graph};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

pub type SharedDataset = Arc<RwLock<Arc<Dataset>>>;

type RpcError = (i32, String);

pub async fn serve(addr: &str, state: SharedDataset) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (sock, _) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            let (rd, mut wr) = sock.into_split();
            let mut lines = BufReader::new(rd).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let resp = handle(&line, &state);
                if wr.write_all(format!("{resp}\n").as_bytes()).await.is_err() {
                    break;
                }
            }
        });
    }
}

pub fn handle(line: &str, state: &SharedDataset) -> Value {
    let Ok(req) = serde_json::from_str::<Value>(line) else {
        return error(Value::Null, -32700, "JSON invalide");
    };
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let data = state.read().unwrap().clone();
    match dispatch(method, &params, &data) {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err((code, msg)) => error(id, code, &msg),
    }
}

fn error(id: Value, code: i32, msg: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": msg}})
}

fn dispatch(method: &str, params: &Value, ds: &Dataset) -> Result<Value, RpcError> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {"name": "open-chiron", "version": env!("CARGO_PKG_VERSION")},
            "capabilities": {"tools": {}}
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list()),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            let out = call_tool(name, &args, ds)?;
            Ok(json!({"content": [{"type": "text", "text": out.to_string()}]}))
        }
        _ => Err((-32601, format!("méthode inconnue: {method}"))),
    }
}

fn tools_list() -> Value {
    let id_schema = json!({"type": "object", "properties": {"id": {"type": "integer"}}, "required": ["id"]});
    json!({"tools": [
        {"name": "graph_stats", "description": "Statistiques globales du graphe",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "get_node", "description": "Propriétés d'un nœud", "inputSchema": id_schema},
        {"name": "neighbors", "description": "Voisins sortants (100 max)", "inputSchema": id_schema},
        {"name": "predict", "description": "Prédiction de Markov après N pas",
         "inputSchema": {"type": "object",
            "properties": {"id": {"type": "integer"}, "steps": {"type": "integer"}},
            "required": ["id"]}}
    ]})
}

fn node_arg(args: &Value, g: &Graph) -> Result<NodeIndex, RpcError> {
    let id = args
        .get("id")
        .and_then(Value::as_u64)
        .ok_or((-32602, "paramètre `id` requis".to_string()))? as usize;
    if id < g.node_count() {
        Ok(NodeIndex::new(id))
    } else {
        Err((-32602, format!("nœud {id} inexistant")))
    }
}

fn call_tool(name: &str, args: &Value, ds: &Dataset) -> Result<Value, RpcError> {
    let g = &ds.graph;
    match name {
        "graph_stats" => Ok(json!({
            "nodes": ds.stats.nodes,
            "edges": ds.stats.edges,
            "density": ds.stats.density,
            "avg_degree": ds.stats.avg_degree,
            "max_out_degree": ds.stats.max_out_degree
        })),
        "get_node" => {
            let n = node_arg(args, g)?;
            Ok(json!({
                "id": n.index(),
                "label": g[n].label,
                "kind": g[n].kind.label(),
                "out_degree": g.neighbors(n).count(),
                "in_degree": g.neighbors_directed(n, Direction::Incoming).count(),
                "reachable": reachable_count(g, n)
            }))
        }
        "neighbors" => {
            let n = node_arg(args, g)?;
            let list: Vec<Value> = g
                .edges(n)
                .take(100)
                .map(|e| json!({"id": e.target().index(), "label": g[e.target()].label, "weight": *e.weight()}))
                .collect();
            Ok(json!(list))
        }
        "predict" => {
            let n = node_arg(args, g)?;
            let steps = args.get("steps").and_then(Value::as_u64).unwrap_or(1).clamp(1, 20) as usize;
            let list: Vec<Value> = markov::predict(g, n, steps, 10)
                .into_iter()
                .map(|p| json!({"id": p.node.index(), "label": g[p.node].label, "probability": p.probability}))
                .collect();
            Ok(json!(list))
        }
        _ => Err((-32601, format!("outil inconnu: {name}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::mock;

    #[test]
    fn json_rpc_roundtrip() {
        let ds = Arc::new(Dataset::new(mock::generate(100, 300, 7)));
        let st: SharedDataset = Arc::new(RwLock::new(ds));
        let r = handle(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"graph_stats","arguments":{}}}"#,
            &st,
        );
        assert_eq!(r["id"], 1);
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("\"nodes\":100"));
        let e = handle(r#"{"jsonrpc":"2.0","id":2,"method":"nope"}"#, &st);
        assert_eq!(e["error"]["code"], -32601);
    }
}