//! Benchmarks for search indexing and query performance.
//!
//! Run with: cargo bench --bench search_bench

use std::time::Instant;
use uuid::Uuid;

use mneme_search::SearchEngine;

fn main() {
    let engine = SearchEngine::in_memory().unwrap();

    // --- Indexing throughput ---
    let note_count = 1000;
    let start = Instant::now();
    for i in 0..note_count {
        engine
            .index_note(
                Uuid::new_v4(),
                &format!("Note about topic {i}"),
                &format!(
                    "This is note number {i}. It contains content about various topics \
                     including programming, systems design, and knowledge management. \
                     The note discusses concepts related to topic {i} in detail.",
                ),
                &[format!("topic-{}", i % 10), format!("batch-{}", i / 100)],
                &format!("notes/topic-{i}.md"),
            )
            .unwrap();
    }
    let index_duration = start.elapsed();
    println!(
        "Indexed {} notes in {:.2}ms ({:.0} notes/sec)",
        note_count,
        index_duration.as_secs_f64() * 1000.0,
        note_count as f64 / index_duration.as_secs_f64()
    );

    // --- Search latency ---
    let queries = [
        "programming",
        "systems design",
        "knowledge management",
        "topic 42",
        "nonexistent term",
    ];

    let iterations = 100;
    for query in &queries {
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = engine.search(query, 10).unwrap();
        }
        let duration = start.elapsed();
        let results = engine.search(query, 10).unwrap();
        println!(
            "Search '{}': {:.2}ms avg ({} results)",
            query,
            duration.as_secs_f64() * 1000.0 / iterations as f64,
            results.len()
        );
    }

    // --- Re-index (update) ---
    let update_id = Uuid::new_v4();
    engine
        .index_note(
            update_id,
            "Original Title",
            "Original content",
            &[],
            "update-test.md",
        )
        .unwrap();

    let start = Instant::now();
    for i in 0..100 {
        engine
            .index_note(
                update_id,
                &format!("Updated Title {i}"),
                &format!("Updated content {i}"),
                &[],
                "update-test.md",
            )
            .unwrap();
    }
    let update_duration = start.elapsed();
    println!(
        "Re-indexed 100 updates in {:.2}ms ({:.2}ms avg)",
        update_duration.as_secs_f64() * 1000.0,
        update_duration.as_secs_f64() * 10.0
    );
}
