//! Knowledge graph types — nodes, edges, and traversal.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: Uuid,
    pub label: String,
    pub kind: NodeKind,
}

/// What kind of entity a graph node represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Note,
    Tag,
    Concept,
}

/// A directed edge in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: Uuid,
    pub target: Uuid,
    pub relation: EdgeRelation,
}

/// The type of relationship an edge represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRelation {
    /// Note links to note (explicit wikilink or markdown link).
    LinksTo,
    /// Note is tagged with a tag.
    TaggedWith,
    /// Note mentions a concept (extracted by AI, Phase 2).
    Mentions,
    /// A custom/user-defined relation.
    Custom(String),
}

/// A subgraph query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subgraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl Subgraph {
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Returns all node IDs directly connected to the given node.
    pub fn neighbors(&self, node_id: Uuid) -> Vec<Uuid> {
        self.edges
            .iter()
            .filter_map(|e| {
                if e.source == node_id {
                    Some(e.target)
                } else if e.target == node_id {
                    Some(e.source)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// A node with computed 2D position for rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutNode {
    pub id: Uuid,
    pub label: String,
    pub kind: NodeKind,
    pub x: f64,
    pub y: f64,
}

/// A fully laid-out graph ready for rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphLayout {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<GraphEdge>,
}

impl GraphLayout {
    /// Compute a force-directed layout from a subgraph.
    ///
    /// Uses a simple spring-embedder: repulsion between all node pairs,
    /// attraction along edges. O(n^2) per iteration, fine for <500 nodes.
    pub fn from_subgraph(subgraph: &Subgraph) -> Self {
        let n = subgraph.nodes.len();
        if n == 0 {
            return Self {
                nodes: Vec::new(),
                edges: subgraph.edges.clone(),
            };
        }

        // Initial positions: place nodes in a circle
        let mut positions: Vec<(f64, f64)> = (0..n)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
                let radius = 50.0;
                (radius * angle.cos(), radius * angle.sin())
            })
            .collect();

        // Build index map: Uuid -> index
        let id_to_idx: std::collections::HashMap<Uuid, usize> = subgraph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.id, i))
            .collect();

        // Edge list as index pairs
        let edge_indices: Vec<(usize, usize)> = subgraph
            .edges
            .iter()
            .filter_map(|e| {
                let s = id_to_idx.get(&e.source)?;
                let t = id_to_idx.get(&e.target)?;
                Some((*s, *t))
            })
            .collect();

        // Force-directed iterations
        let iterations = 100;
        let repulsion = 2000.0;
        let attraction = 0.01;
        let damping = 0.9;
        let mut velocities: Vec<(f64, f64)> = vec![(0.0, 0.0); n];

        for _ in 0..iterations {
            let mut forces: Vec<(f64, f64)> = vec![(0.0, 0.0); n];

            // Repulsion between all pairs (Coulomb's law)
            for i in 0..n {
                for j in (i + 1)..n {
                    let dx = positions[i].0 - positions[j].0;
                    let dy = positions[i].1 - positions[j].1;
                    let dist_sq = dx * dx + dy * dy;
                    let dist = dist_sq.sqrt().max(1.0);
                    let force = repulsion / dist_sq.max(1.0);
                    let fx = force * dx / dist;
                    let fy = force * dy / dist;
                    forces[i].0 += fx;
                    forces[i].1 += fy;
                    forces[j].0 -= fx;
                    forces[j].1 -= fy;
                }
            }

            // Attraction along edges (Hooke's law)
            for &(s, t) in &edge_indices {
                let dx = positions[t].0 - positions[s].0;
                let dy = positions[t].1 - positions[s].1;
                let fx = attraction * dx;
                let fy = attraction * dy;
                forces[s].0 += fx;
                forces[s].1 += fy;
                forces[t].0 -= fx;
                forces[t].1 -= fy;
            }

            // Apply forces with velocity damping
            for i in 0..n {
                velocities[i].0 = (velocities[i].0 + forces[i].0) * damping;
                velocities[i].1 = (velocities[i].1 + forces[i].1) * damping;
                // Clamp velocity
                let max_vel = 10.0;
                velocities[i].0 = velocities[i].0.clamp(-max_vel, max_vel);
                velocities[i].1 = velocities[i].1.clamp(-max_vel, max_vel);
                positions[i].0 += velocities[i].0;
                positions[i].1 += velocities[i].1;
            }
        }

        let nodes = subgraph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| LayoutNode {
                id: node.id,
                label: node.label.clone(),
                kind: node.kind,
                x: positions[i].0,
                y: positions[i].1,
            })
            .collect();

        Self {
            nodes,
            edges: subgraph.edges.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subgraph_neighbors() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let graph = Subgraph {
            nodes: vec![
                GraphNode {
                    id: a,
                    label: "A".into(),
                    kind: NodeKind::Note,
                },
                GraphNode {
                    id: b,
                    label: "B".into(),
                    kind: NodeKind::Note,
                },
                GraphNode {
                    id: c,
                    label: "C".into(),
                    kind: NodeKind::Tag,
                },
            ],
            edges: vec![
                GraphEdge {
                    source: a,
                    target: b,
                    relation: EdgeRelation::LinksTo,
                },
                GraphEdge {
                    source: a,
                    target: c,
                    relation: EdgeRelation::TaggedWith,
                },
            ],
        };

        let neighbors = graph.neighbors(a);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&b));
        assert!(neighbors.contains(&c));
    }

    #[test]
    fn subgraph_neighbors_bidirectional() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let graph = Subgraph {
            nodes: vec![],
            edges: vec![GraphEdge {
                source: a,
                target: b,
                relation: EdgeRelation::LinksTo,
            }],
        };

        // b should see a as a neighbor (reverse direction)
        let neighbors = graph.neighbors(b);
        assert_eq!(neighbors, vec![a]);
    }
}
