use std::{
    collections::HashMap, convert::Infallible, ffi::OsStr, hint::black_box, path::PathBuf,
    sync::Arc,
};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_task::{Session, SessionCallbacks, plan_request::SyntheticPlanRequest};

/// A no-op task synthesizer for benchmarking purposes.
#[derive(Debug, Default)]
struct NoOpTaskSynthesizer;

#[async_trait::async_trait(?Send)]
impl vite_task::TaskSynthesizer<Infallible> for NoOpTaskSynthesizer {
    fn should_synthesize_for_program(&self, _program: &str) -> bool {
        false
    }

    async fn synthesize_task(
        &mut self,
        subcommand: Infallible,
        _envs: &Arc<HashMap<Arc<OsStr>, Arc<OsStr>>>,
        _cwd: &Arc<AbsolutePath>,
    ) -> anyhow::Result<SyntheticPlanRequest> {
        match subcommand {}
    }
}

/// Owned session callbacks for benchmarking.
#[derive(Default)]
struct BenchSessionCallbacks {
    task_synthesizer: NoOpTaskSynthesizer,
    user_config_loader: vite_task::loader::JsonUserConfigLoader,
}

impl BenchSessionCallbacks {
    fn as_callbacks(&mut self) -> SessionCallbacks<'_, Infallible> {
        SessionCallbacks {
            task_synthesizer: &mut self.task_synthesizer,
            user_config_loader: &mut self.user_config_loader,
        }
    }
}

fn bench_workspace_load(c: &mut Criterion) {
    let fixture_path = AbsolutePathBuf::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .unwrap()
        .join("fixtures")
        .join("monorepo");

    let runtime = Runtime::new().unwrap();

    // Session::ensure_task_graph_loaded benchmark
    let mut session_group = c.benchmark_group("session_task_graph_load");
    session_group.measurement_time(std::time::Duration::from_secs(10));

    session_group.bench_function("ensure_task_graph_loaded", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let mut owned_callbacks = BenchSessionCallbacks::default();
                let envs: HashMap<Arc<OsStr>, Arc<OsStr>> = HashMap::new();
                let mut session = Session::init_with(
                    envs,
                    fixture_path.clone().into(),
                    owned_callbacks.as_callbacks(),
                )
                .expect("Failed to create session");
                black_box(
                    session.ensure_task_graph_loaded().await.expect("Failed to load task graph"),
                );
            });
        });
    });

    session_group.bench_with_input(BenchmarkId::new("packages", 100), &fixture_path, |b, path| {
        b.iter(|| {
            runtime.block_on(async {
                let mut owned_callbacks = BenchSessionCallbacks::default();
                let envs: HashMap<Arc<OsStr>, Arc<OsStr>> = HashMap::new();
                let mut session =
                    Session::init_with(envs, path.clone().into(), owned_callbacks.as_callbacks())
                        .expect("Failed to create session");
                black_box(
                    session.ensure_task_graph_loaded().await.expect("Failed to load task graph"),
                );
            });
        });
    });

    session_group.finish();
}

criterion_group!(benches, bench_workspace_load);
criterion_main!(benches);
