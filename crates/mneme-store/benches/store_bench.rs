//! Benchmarks for vault operations.

use criterion::{Criterion, criterion_group, criterion_main};
use mneme_core::note::CreateNote;
use mneme_store::Vault;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn bench_create_note(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let dir = TempDir::new().unwrap();
    let vault = rt.block_on(Vault::open(dir.path())).unwrap();

    let mut counter = 0u64;
    c.bench_function("create_note", |b| {
        b.iter(|| {
            counter += 1;
            rt.block_on(vault.create_note(CreateNote {
                title: format!("Bench Note {counter}"),
                path: None,
                content: "Benchmark content for testing note creation performance.".into(),
                tags: vec!["bench".into()],
            }))
            .unwrap();
        });
    });
}

fn bench_list_notes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let dir = TempDir::new().unwrap();
    let vault = rt.block_on(Vault::open(dir.path())).unwrap();

    // Pre-populate
    for i in 0..200 {
        rt.block_on(vault.create_note(CreateNote {
            title: format!("Note {i}"),
            path: None,
            content: format!("Content {i}"),
            tags: vec![],
        }))
        .unwrap();
    }

    c.bench_function("list_50_of_200_notes", |b| {
        b.iter(|| {
            rt.block_on(vault.list_notes(50, 0)).unwrap();
        });
    });
}

criterion_group!(benches, bench_create_note, bench_list_notes);
criterion_main!(benches);
