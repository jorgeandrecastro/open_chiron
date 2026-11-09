# Open Chiron

Moteur souverain d'analyse de données complexes et de visualisation de graphes, écrit en Rust.
Application desktop haute performance, conçue pour manipuler des graphes de plus de 100 000 nœuds
et être pilotée par un agent IA externe via une API locale MCP.

## Fonctionnalités

- **Visualisation** : canvas interactif (pan, zoom centré sur le curseur, sélection) avec échantillonnage et culling pour rester fluide sur de très gros graphes.
- **Ingestion asynchrone** : génération de données synthétiques (100 000+ nœuds) ou import CSV, sans bloquer l'interface (tokio).
- **Analyse probabiliste** : prédiction de transitions par chaîne de Markov locale (ndarray) à partir du nœud sélectionné.
- **API MCP locale** : état du graphe exposé en JSON-RPC 2.0 pour un agent IA.
- **Interface épurée** : thèmes clair et sombre, coins arrondis, trois panneaux (ingestion et statistiques, graphe, propriétés et prédictions).

## Stack

Rust 2021 · eframe/egui 0.29 · petgraph · ndarray · tokio · serde_json

## Architecture

```
src/
├── main.rs        point d'entrée : runtime tokio, serveur MCP, fenêtre eframe
├── graph/         structure petgraph, statistiques, layout, générateur mock, import CSV
├── analytics/     chaînes de Markov (matrice de transition locale, prédiction à N pas)
├── mcp/           serveur JSON-RPC 2.0 (TCP, un message JSON par ligne)
└── gui/           application egui : thème, panneaux, canvas interactif
```

## Lancer

```bash
cargo run --release
```

Au démarrage, un graphe de démonstration de 600 nœuds est chargé. Le panneau de gauche permet de
générer jusqu'à 500 000 nœuds ou d'importer un fichier d'arêtes.

### Format CSV

```
source,cible,poids
alice,bob,0.8
bob,carol
```

Séparateurs `,` `;` ou tabulation. Poids optionnel (1.0 par défaut). Les lignes vides, commentées (`#`) et l'en-tête sont ignorées.

## Tests et mesures de performance

```bash
cargo test --release -- --nocapture
```

Les tests d'effort affichent les temps d'ingestion, de parcours en largeur, de scan des degrés
et de prédiction Markov sur un graphe de 100 000 nœuds et 400 000 relations.

## API MCP

Serveur JSON-RPC 2.0 sur `127.0.0.1:7878` (modifiable via `CHIRON_MCP_ADDR`), un message JSON par ligne.

| Méthode      | Description                                  |
|--------------|----------------------------------------------|
| `initialize` | Informations serveur et capacités            |
| `tools/list` | Liste des outils                             |
| `tools/call` | Appel d'un outil (ci-dessous)                |

Outils : `graph_stats`, `get_node {id}`, `neighbors {id}`, `predict {id, steps}`.

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"graph_stats","arguments":{}}}' \
  | nc 127.0.0.1 7878
```

Le transport est du TCP brut ; un pont stdio ou HTTP sera nécessaire pour les clients MCP standard.

## Feuille de route

- [ ] Persistance du graphe (sled)
- [ ] PageRank et détection de communautés
- [ ] Layout Barnes-Hut / GPU pour les très gros graphes
- [ ] Transport MCP stdio et HTTP
- [ ] Import de formats riches (JSON, GraphML, Parquet)

## Licence

GPL-2.0-or-later