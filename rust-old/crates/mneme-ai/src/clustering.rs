//! Schema clustering — discover emergent topic structure in the vault.
//!
//! Uses K-means++ on note embeddings to group notes into topical clusters,
//! with optional LLM labeling via daimon.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A cluster of related notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: usize,
    /// LLM-generated label, or a default like "Cluster 1".
    pub label: String,
    /// One-line summary of the cluster's theme.
    pub summary: String,
    /// Note IDs in this cluster.
    pub note_ids: Vec<Uuid>,
    /// Note titles for display convenience.
    pub note_titles: Vec<String>,
    /// Inertia (sum of squared distances to centroid) for this cluster.
    pub inertia: f64,
}

/// Result of a clustering pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusteringResult {
    pub k: usize,
    pub total_notes: usize,
    pub total_inertia: f64,
    pub clusters: Vec<Cluster>,
}

/// A note embedding ready for clustering.
pub struct NoteEmbedding {
    pub id: Uuid,
    pub title: String,
    pub embedding: Vec<f32>,
}

/// Run K-means++ clustering on note embeddings.
///
/// If `k` is None, uses the elbow heuristic to choose k (2..=max_k).
pub fn cluster_notes(notes: &[NoteEmbedding], k: Option<usize>, max_k: usize) -> ClusteringResult {
    if notes.len() < 2 {
        return ClusteringResult {
            k: 1,
            total_notes: notes.len(),
            total_inertia: 0.0,
            clusters: vec![Cluster {
                id: 0,
                label: "All Notes".into(),
                summary: String::new(),
                note_ids: notes.iter().map(|n| n.id).collect(),
                note_titles: notes.iter().map(|n| n.title.clone()).collect(),
                inertia: 0.0,
            }],
        };
    }

    let chosen_k = match k {
        Some(k) => k.min(notes.len()).max(2),
        None => find_elbow(notes, max_k.min(notes.len())),
    };

    let assignments = kmeans_pp(notes, chosen_k, 50);
    build_result(notes, &assignments, chosen_k)
}

/// K-means++ initialization + Lloyd's iteration.
fn kmeans_pp(notes: &[NoteEmbedding], k: usize, max_iters: usize) -> Vec<usize> {
    let dim = notes[0].embedding.len();
    let n = notes.len();

    // --- K-means++ initialization ---
    let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(k);

    // Deterministic first centroid: pick the note closest to the mean
    let mean = compute_mean_all(notes, dim);
    let first = (0..n)
        .min_by(|&a, &b| {
            let da = squared_distance(&notes[a].embedding, &mean);
            let db = squared_distance(&notes[b].embedding, &mean);
            da.partial_cmp(&db).unwrap()
        })
        .unwrap();
    centroids.push(notes[first].embedding.clone());

    // Subsequent centroids: pick the point with max min-distance to existing centroids
    // (deterministic variant of K-means++ — avoids RNG dependency)
    for _ in 1..k {
        let mut best_idx = 0;
        let mut best_dist = f64::MIN;
        for (i, note) in notes.iter().enumerate() {
            let min_dist = centroids
                .iter()
                .map(|c| squared_distance(&note.embedding, c))
                .fold(f64::MAX, f64::min);
            if min_dist > best_dist {
                best_dist = min_dist;
                best_idx = i;
            }
        }
        centroids.push(notes[best_idx].embedding.clone());
    }

    // --- Lloyd's iteration ---
    let mut assignments = vec![0usize; n];

    for _ in 0..max_iters {
        // Assign each point to nearest centroid
        let mut changed = false;
        for (i, note) in notes.iter().enumerate() {
            let nearest = (0..k)
                .min_by(|&a, &b| {
                    let da = squared_distance(&note.embedding, &centroids[a]);
                    let db = squared_distance(&note.embedding, &centroids[b]);
                    da.partial_cmp(&db).unwrap()
                })
                .unwrap();
            if assignments[i] != nearest {
                assignments[i] = nearest;
                changed = true;
            }
        }

        if !changed {
            break;
        }

        // Recompute centroids
        let mut sums = vec![vec![0.0f64; dim]; k];
        let mut counts = vec![0usize; k];

        for (i, note) in notes.iter().enumerate() {
            let c = assignments[i];
            counts[c] += 1;
            for (d, sum) in sums[c].iter_mut().enumerate() {
                *sum += note.embedding[d] as f64;
            }
        }

        for c in 0..k {
            if counts[c] > 0 {
                for d in 0..dim {
                    centroids[c][d] = (sums[c][d] / counts[c] as f64) as f32;
                }
            }
        }
    }

    assignments
}

/// Find the best k using the elbow heuristic.
///
/// Computes inertia for k=2..=max_k, picks the k where the second derivative
/// (rate of improvement change) is maximized.
fn find_elbow(notes: &[NoteEmbedding], max_k: usize) -> usize {
    let max_k = max_k.clamp(2, 10); // cap search range
    if notes.len() <= 3 {
        return 2;
    }

    let mut inertias = Vec::with_capacity(max_k);
    for k in 2..=max_k {
        let assignments = kmeans_pp(notes, k, 30);
        let inertia = compute_inertia(notes, &assignments, k);
        inertias.push(inertia);
    }

    if inertias.len() < 3 {
        return 2;
    }

    // Find elbow: maximum second derivative
    let mut best_k = 2;
    let mut best_diff = f64::MIN;
    for i in 1..inertias.len() - 1 {
        let second_deriv = (inertias[i - 1] - inertias[i]) - (inertias[i] - inertias[i + 1]);
        if second_deriv > best_diff {
            best_diff = second_deriv;
            best_k = i + 2; // k = index + 2 since we start from k=2
        }
    }

    best_k
}

fn compute_inertia(notes: &[NoteEmbedding], assignments: &[usize], k: usize) -> f64 {
    let dim = notes[0].embedding.len();

    // Compute centroids
    let mut sums = vec![vec![0.0f64; dim]; k];
    let mut counts = vec![0usize; k];
    for (i, note) in notes.iter().enumerate() {
        let c = assignments[i];
        counts[c] += 1;
        for (d, sum) in sums[c].iter_mut().enumerate() {
            *sum += note.embedding[d] as f64;
        }
    }
    let centroids: Vec<Vec<f32>> = sums
        .iter()
        .zip(counts.iter())
        .map(|(s, &cnt)| {
            if cnt == 0 {
                vec![0.0f32; dim]
            } else {
                s.iter().map(|v| (*v / cnt as f64) as f32).collect()
            }
        })
        .collect();

    // Sum of squared distances
    notes
        .iter()
        .zip(assignments.iter())
        .map(|(note, &c)| squared_distance(&note.embedding, &centroids[c]))
        .sum()
}

fn build_result(notes: &[NoteEmbedding], assignments: &[usize], k: usize) -> ClusteringResult {
    let dim = notes[0].embedding.len();

    // Compute centroids for inertia
    let mut sums = vec![vec![0.0f64; dim]; k];
    let mut counts = vec![0usize; k];
    for (i, note) in notes.iter().enumerate() {
        let c = assignments[i];
        counts[c] += 1;
        for (d, sum) in sums[c].iter_mut().enumerate() {
            *sum += note.embedding[d] as f64;
        }
    }
    let centroids: Vec<Vec<f32>> = sums
        .iter()
        .zip(counts.iter())
        .map(|(s, &cnt)| {
            if cnt == 0 {
                vec![0.0f32; dim]
            } else {
                s.iter().map(|v| (*v / cnt as f64) as f32).collect()
            }
        })
        .collect();

    let mut clusters: Vec<Cluster> = (0..k)
        .map(|c| Cluster {
            id: c,
            label: format!("Cluster {}", c + 1),
            summary: String::new(),
            note_ids: Vec::new(),
            note_titles: Vec::new(),
            inertia: 0.0,
        })
        .collect();

    for (i, note) in notes.iter().enumerate() {
        let c = assignments[i];
        clusters[c].note_ids.push(note.id);
        clusters[c].note_titles.push(note.title.clone());
        clusters[c].inertia += squared_distance(&note.embedding, &centroids[c]);
    }

    // Remove empty clusters
    clusters.retain(|c| !c.note_ids.is_empty());

    let total_inertia = clusters.iter().map(|c| c.inertia).sum();

    ClusteringResult {
        k: clusters.len(),
        total_notes: notes.len(),
        total_inertia,
        clusters,
    }
}

fn squared_distance(a: &[f32], b: &[f32]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = (*x as f64) - (*y as f64);
            d * d
        })
        .sum()
}

fn compute_mean_all(notes: &[NoteEmbedding], dim: usize) -> Vec<f32> {
    let n = notes.len() as f64;
    let mut mean = vec![0.0f64; dim];
    for note in notes {
        for (d, m) in mean.iter_mut().enumerate() {
            *m += note.embedding[d] as f64;
        }
    }
    mean.iter().map(|v| (*v / n) as f32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_embedding(id: Uuid, title: &str, values: Vec<f32>) -> NoteEmbedding {
        NoteEmbedding {
            id,
            title: title.into(),
            embedding: values,
        }
    }

    #[test]
    fn cluster_single_note() {
        let notes = vec![make_embedding(Uuid::new_v4(), "Solo", vec![1.0, 0.0, 0.0])];
        let result = cluster_notes(&notes, None, 5);
        assert_eq!(result.k, 1);
        assert_eq!(result.clusters[0].note_ids.len(), 1);
    }

    #[test]
    fn cluster_two_groups() {
        let mut notes = Vec::new();
        // Group A: points near (1, 0, 0)
        for i in 0..5 {
            notes.push(make_embedding(
                Uuid::new_v4(),
                &format!("A{i}"),
                vec![1.0, 0.01 * i as f32, 0.0],
            ));
        }
        // Group B: points near (0, 1, 0)
        for i in 0..5 {
            notes.push(make_embedding(
                Uuid::new_v4(),
                &format!("B{i}"),
                vec![0.0, 1.0, 0.01 * i as f32],
            ));
        }

        let result = cluster_notes(&notes, Some(2), 5);
        assert_eq!(result.k, 2);
        assert_eq!(result.total_notes, 10);

        // Each cluster should have 5 notes
        let mut sizes: Vec<usize> = result.clusters.iter().map(|c| c.note_ids.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![5, 5]);
    }

    #[test]
    fn cluster_with_elbow() {
        let mut notes = Vec::new();
        for i in 0..6 {
            notes.push(make_embedding(
                Uuid::new_v4(),
                &format!("A{i}"),
                vec![1.0, 0.01 * i as f32, 0.0],
            ));
        }
        for i in 0..6 {
            notes.push(make_embedding(
                Uuid::new_v4(),
                &format!("B{i}"),
                vec![0.0, 1.0, 0.01 * i as f32],
            ));
        }

        let result = cluster_notes(&notes, None, 8);
        assert!(result.k >= 2);
        assert_eq!(result.total_notes, 12);
    }

    #[test]
    fn empty_input() {
        let result = cluster_notes(&[], None, 5);
        assert_eq!(result.k, 1);
        assert_eq!(result.total_notes, 0);
    }

    #[test]
    fn squared_distance_identical() {
        assert_eq!(squared_distance(&[1.0, 2.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn squared_distance_values() {
        let d = squared_distance(&[0.0, 0.0], &[3.0, 4.0]);
        assert!((d - 25.0).abs() < 1e-6);
    }

    #[test]
    fn three_clear_clusters() {
        let mut notes = Vec::new();
        for i in 0..4 {
            notes.push(make_embedding(
                Uuid::new_v4(),
                &format!("X{i}"),
                vec![10.0 + 0.01 * i as f32, 0.0, 0.0],
            ));
        }
        for i in 0..4 {
            notes.push(make_embedding(
                Uuid::new_v4(),
                &format!("Y{i}"),
                vec![0.0, 10.0 + 0.01 * i as f32, 0.0],
            ));
        }
        for i in 0..4 {
            notes.push(make_embedding(
                Uuid::new_v4(),
                &format!("Z{i}"),
                vec![0.0, 0.0, 10.0 + 0.01 * i as f32],
            ));
        }

        let result = cluster_notes(&notes, Some(3), 5);
        assert_eq!(result.k, 3);
        for cluster in &result.clusters {
            assert_eq!(cluster.note_ids.len(), 4);
        }
    }

    #[test]
    fn inertia_decreases_with_more_clusters() {
        let mut notes = Vec::new();
        for i in 0..20 {
            let x = (i as f32 * 0.5).cos();
            let y = (i as f32 * 0.5).sin();
            notes.push(make_embedding(
                Uuid::new_v4(),
                &format!("N{i}"),
                vec![x, y, 0.0],
            ));
        }

        let r2 = cluster_notes(&notes, Some(2), 5);
        let r5 = cluster_notes(&notes, Some(5), 5);
        assert!(r5.total_inertia <= r2.total_inertia);
    }
}
