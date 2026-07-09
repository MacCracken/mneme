//! Benchmarks for search indexing and query performance.

use criterion::{Criterion, criterion_group, criterion_main};
use mneme_search::SearchEngine;
use uuid::Uuid;

fn bench_index_note(c: &mut Criterion) {
    let engine = SearchEngine::in_memory().unwrap();
    let id = Uuid::new_v4();

    c.bench_function("index_single_note", |b| {
        b.iter(|| {
            engine
                .index_note(
                    id,
                    "Benchmark Note Title",
                    "This is the content of a benchmark note. It contains several sentences about Rust programming, systems design, and software architecture.",
                    &["benchmark".to_string(), "rust".to_string()],
                    "bench/note.md",
                )
                .unwrap();
        });
    });
}

fn bench_index_many_notes(c: &mut Criterion) {
    let engine = SearchEngine::in_memory().unwrap();

    c.bench_function("index_100_notes", |b| {
        b.iter(|| {
            for i in 0..100 {
                let id = Uuid::new_v4();
                engine
                    .index_note(
                        id,
                        &format!("Note {i}: Systems Programming"),
                        &format!("Content for note {i}. Discusses topics like memory management, concurrency, and type systems in the context of Rust programming."),
                        &["bench".to_string()],
                        &format!("bench/note-{i}.md"),
                    )
                    .unwrap();
            }
        });
    });
}

fn bench_search_query(c: &mut Criterion) {
    let engine = SearchEngine::in_memory().unwrap();

    // Pre-populate index
    for i in 0..500 {
        let id = Uuid::new_v4();
        let topics = ["rust", "python", "javascript", "systems", "web", "database"];
        let topic = topics[i % topics.len()];
        engine
            .index_note(
                id,
                &format!("{topic} note {i}"),
                &format!("Deep dive into {topic} programming. Note number {i} covers advanced concepts in {topic} development including best practices and patterns."),
                &[topic.to_string()],
                &format!("notes/{topic}/{i}.md"),
            )
            .unwrap();
    }

    c.bench_function("search_500_notes", |b| {
        b.iter(|| {
            engine.search("rust programming", 20).unwrap();
        });
    });

    c.bench_function("search_no_results", |b| {
        b.iter(|| {
            engine.search("xyznonexistent", 20).unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_index_note,
    bench_index_many_notes,
    bench_search_query
);
criterion_main!(benches);
