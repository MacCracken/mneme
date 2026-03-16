//! Advanced query DSL — parse rich queries for the AI shell and API.
//!
//! Supports temporal filters, boolean tag expressions, graph traversal,
//! and bulk operations. Falls back to plain text search for unrecognized input.
//!
//! # Syntax
//!
//! ```text
//! notes edited last week about Rust
//! notes tagged #project AND NOT #archived
//! notes connected to "Design Patterns" within 2 hops
//! stale notes older than 6 months
//! ```

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// A parsed structured query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredQuery {
    /// Free-text search terms (empty if purely structured).
    pub text: String,
    /// Tag filters.
    pub tags: TagFilter,
    /// Temporal filter.
    pub temporal: Option<TemporalFilter>,
    /// Graph traversal.
    pub graph: Option<GraphFilter>,
    /// Result limit.
    pub limit: Option<usize>,
    /// Whether this targets stale notes specifically.
    pub stale_only: bool,
}

/// Tag-based boolean filter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TagFilter {
    /// Tags that must be present (AND).
    pub include: Vec<String>,
    /// Tags that must NOT be present.
    pub exclude: Vec<String>,
}

/// Temporal filter on note timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalFilter {
    /// Filter field: "edited", "created", or "accessed".
    pub field: String,
    /// Notes after this date.
    pub after: Option<DateTime<Utc>>,
    /// Notes before this date.
    pub before: Option<DateTime<Utc>>,
}

/// Graph traversal filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphFilter {
    /// Origin note title or ID.
    pub origin: String,
    /// Maximum number of hops.
    pub max_hops: usize,
}

/// Parse a natural language query into a structured query.
///
/// Extracts structured components and leaves the remainder as free text.
pub fn parse_query(input: &str) -> StructuredQuery {
    let mut query = StructuredQuery {
        text: String::new(),
        tags: TagFilter::default(),
        temporal: None,
        graph: None,
        limit: None,
        stale_only: false,
    };

    let input = input.trim();
    if input.is_empty() {
        return query;
    }

    let mut remaining_words: Vec<&str> = Vec::new();
    let words: Vec<&str> = input.split_whitespace().collect();
    let mut i = 0;

    while i < words.len() {
        let word = words[i];
        let lower = word.to_lowercase();

        // Tag includes: #tag or "tagged #tag"
        if word.starts_with('#') {
            let tag = word.trim_start_matches('#');
            if !tag.is_empty() {
                query.tags.include.push(tag.to_string());
            }
            i += 1;
            continue;
        }

        // NOT #tag
        if (lower == "not" || lower == "!") && i + 1 < words.len() && words[i + 1].starts_with('#') {
            let tag = words[i + 1].trim_start_matches('#');
            if !tag.is_empty() {
                query.tags.exclude.push(tag.to_string());
            }
            i += 2;
            continue;
        }

        // "tagged" keyword followed by tags
        if lower == "tagged" && i + 1 < words.len() && words[i + 1].starts_with('#') {
            i += 1; // skip "tagged", the #tag will be picked up next iteration
            continue;
        }

        // Skip boolean operators
        if lower == "and" || lower == "or" {
            i += 1;
            continue;
        }

        // Temporal: "last week", "last month", "last N days"
        if lower == "last" && i + 1 < words.len() {
            let period = words[i + 1].to_lowercase();
            let duration = match period.as_str() {
                "week" => Some(Duration::days(7)),
                "month" => Some(Duration::days(30)),
                "year" => Some(Duration::days(365)),
                _ => {
                    // "last N days/weeks/months"
                    if let Ok(n) = period.parse::<i64>() {
                        if i + 2 < words.len() {
                            let unit = words[i + 2].to_lowercase();
                            let d = match unit.trim_end_matches('s').as_ref() {
                                "day" => Some(Duration::days(n)),
                                "week" => Some(Duration::weeks(n)),
                                "month" => Some(Duration::days(n * 30)),
                                _ => None,
                            };
                            if d.is_some() {
                                i += 1; // skip the unit word too
                            }
                            d
                        } else {
                            Some(Duration::days(n))
                        }
                    } else {
                        None
                    }
                }
            };

            if let Some(dur) = duration {
                let field = detect_temporal_field(&words, i);
                query.temporal = Some(TemporalFilter {
                    field,
                    after: Some(Utc::now() - dur),
                    before: None,
                });
                i += 2;
                continue;
            }
        }

        // "older than N days/months"
        if lower == "older" && i + 2 < words.len() && words[i + 1].to_lowercase() == "than" {
            if let Ok(n) = words[i + 2].parse::<i64>() {
                let unit = if i + 3 < words.len() {
                    words[i + 3].to_lowercase()
                } else {
                    "days".into()
                };
                let dur = match unit.trim_end_matches('s') {
                    "day" => Duration::days(n),
                    "week" => Duration::weeks(n),
                    "month" => Duration::days(n * 30),
                    "year" => Duration::days(n * 365),
                    _ => Duration::days(n),
                };
                query.temporal = Some(TemporalFilter {
                    field: "edited".into(),
                    after: None,
                    before: Some(Utc::now() - dur),
                });
                i += 4;
                continue;
            }
        }

        // "stale" keyword
        if lower == "stale" {
            query.stale_only = true;
            i += 1;
            continue;
        }

        // Graph: "connected to X within N hops"
        if lower == "connected" && i + 2 < words.len() && words[i + 1].to_lowercase() == "to" {
            let origin = words[i + 2].trim_matches('"').to_string();
            let mut hops = 2; // default
            if i + 4 < words.len()
                && words[i + 3].to_lowercase() == "within"
            {
                if let Ok(n) = words[i + 4].parse::<usize>() {
                    hops = n;
                    i += 3; // skip "within N hops"
                    if i + 1 < words.len() && words[i].to_lowercase() == "hops" {
                        i += 1;
                    }
                }
            }
            query.graph = Some(GraphFilter {
                origin,
                max_hops: hops,
            });
            i += 3;
            continue;
        }

        // "limit N"
        if lower == "limit" && i + 1 < words.len() {
            if let Ok(n) = words[i + 1].parse::<usize>() {
                query.limit = Some(n);
                i += 2;
                continue;
            }
        }

        // Skip noise words
        if matches!(lower.as_str(), "notes" | "about" | "from" | "the" | "with" | "edited" | "created" | "accessed") {
            // "edited/created/accessed" are handled by temporal context
            i += 1;
            continue;
        }

        // Everything else is free text
        remaining_words.push(word);
        i += 1;
    }

    query.text = remaining_words.join(" ");
    query
}

/// Detect which timestamp field is being referenced in the surrounding words.
fn detect_temporal_field(words: &[&str], around_idx: usize) -> String {
    let start = around_idx.saturating_sub(3);
    let end = (around_idx + 3).min(words.len());
    for i in start..end {
        let w = words[i].to_lowercase();
        if w == "edited" || w == "updated" || w == "modified" {
            return "edited".into();
        }
        if w == "created" || w == "made" || w == "new" {
            return "created".into();
        }
        if w == "accessed" || w == "viewed" || w == "read" || w == "opened" {
            return "accessed".into();
        }
    }
    "edited".into() // default
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_query() {
        let q = parse_query("rust ownership borrowing");
        assert_eq!(q.text, "rust ownership borrowing");
        assert!(q.tags.include.is_empty());
        assert!(q.temporal.is_none());
    }

    #[test]
    fn tag_include() {
        let q = parse_query("notes tagged #project AND #rust");
        assert!(q.tags.include.contains(&"project".to_string()));
        assert!(q.tags.include.contains(&"rust".to_string()));
    }

    #[test]
    fn tag_exclude() {
        let q = parse_query("#project NOT #archived");
        assert!(q.tags.include.contains(&"project".to_string()));
        assert!(q.tags.exclude.contains(&"archived".to_string()));
    }

    #[test]
    fn temporal_last_week() {
        let q = parse_query("notes edited last week about Rust");
        assert!(q.temporal.is_some());
        let t = q.temporal.unwrap();
        assert_eq!(t.field, "edited");
        assert!(t.after.is_some());
        assert_eq!(q.text, "Rust");
    }

    #[test]
    fn temporal_last_n_days() {
        let q = parse_query("notes last 30 days");
        assert!(q.temporal.is_some());
        let t = q.temporal.unwrap();
        assert!(t.after.is_some());
    }

    #[test]
    fn temporal_older_than() {
        let q = parse_query("stale notes older than 6 months");
        assert!(q.stale_only);
        assert!(q.temporal.is_some());
        let t = q.temporal.unwrap();
        assert!(t.before.is_some());
    }

    #[test]
    fn graph_query() {
        let q = parse_query("notes connected to Design within 3 hops");
        assert!(q.graph.is_some());
        let g = q.graph.unwrap();
        assert_eq!(g.origin, "Design");
        assert_eq!(g.max_hops, 3);
    }

    #[test]
    fn combined_query() {
        let q = parse_query("#rust NOT #archived last month Rust patterns");
        assert!(q.tags.include.contains(&"rust".to_string()));
        assert!(q.tags.exclude.contains(&"archived".to_string()));
        assert!(q.temporal.is_some());
        assert_eq!(q.text, "Rust patterns");
    }

    #[test]
    fn empty_query() {
        let q = parse_query("");
        assert!(q.text.is_empty());
        assert!(q.tags.include.is_empty());
        assert!(q.temporal.is_none());
    }

    #[test]
    fn limit_query() {
        let q = parse_query("rust limit 5");
        assert_eq!(q.limit, Some(5));
        assert_eq!(q.text, "rust");
    }

    #[test]
    fn stale_keyword() {
        let q = parse_query("stale notes");
        assert!(q.stale_only);
    }
}
