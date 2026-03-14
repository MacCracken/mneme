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
