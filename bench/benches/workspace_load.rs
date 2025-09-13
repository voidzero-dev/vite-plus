use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::{hint::black_box, path::PathBuf};
use vite_path::AbsolutePathBuf;
use vite_task::Workspace;

fn bench_workspace_load(c: &mut Criterion) {
    let fixture_path = AbsolutePathBuf::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .unwrap()
        .join("fixtures")
        .join("monorepo");

    // Basic workspace load benchmark
    c.bench_function("workspace_load_1000_packages", |b| {
        b.iter(|| {
            let workspace = black_box(Workspace::load(fixture_path.clone(), true))
                .expect("Failed to load workspace");
            black_box(workspace);
        });
    });

    // Benchmark group for more detailed analysis
    let mut group = c.benchmark_group("workspace_load_detailed");

    group.measurement_time(std::time::Duration::from_secs(10));

    // Benchmark just the load operation
    group.bench_function("basic_load", |b| {
        b.iter(|| {
            let workspace = Workspace::load(black_box(fixture_path.clone()), true)
                .expect("Failed to load workspace");
            black_box(workspace);
        });
    });

    // Benchmark load with cache verification
    group.bench_function("load_with_cache_path", |b| {
        let cache_path = fixture_path.join("node_modules/.vite/task-cache");
        b.iter(|| {
            let workspace = black_box(
                Workspace::load_with_cache_path(
                    fixture_path.clone(),
                    Some(cache_path.clone()),
                    true,
                )
                .expect("Failed to load workspace"),
            );
            black_box(workspace);
        });
    });

    group.finish();

    // Benchmark different monorepo sizes
    let mut size_group = c.benchmark_group("workspace_load_by_size");
    size_group.sample_size(20);

    // We only have the 100-package fixture, but we can still use it
    size_group.bench_with_input(BenchmarkId::new("packages", 100), &fixture_path, |b, path| {
        b.iter(|| {
            let workspace =
                Workspace::load(black_box(path.clone()), true).expect("Failed to load workspace");
            black_box(workspace);
        });
    });

    size_group.finish();
}

criterion_group!(benches, bench_workspace_load);
criterion_main!(benches);
