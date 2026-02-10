use std::{ffi::OsStr, hint::black_box, path::PathBuf, sync::Arc};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rustc_hash::FxHashMap;
use tokio::runtime::Runtime;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_task::{
    CommandHandler, HandledCommand, Session, SessionCallbacks, plan_request::ScriptCommand,
};

/// A no-op command handler for benchmarking purposes.
#[derive(Debug, Default)]
struct NoOpCommandHandler;

#[async_trait::async_trait(?Send)]
impl CommandHandler for NoOpCommandHandler {
    async fn handle_command(
        &mut self,
        _command: &mut ScriptCommand,
    ) -> anyhow::Result<HandledCommand> {
        Ok(HandledCommand::Verbatim)
    }
}

/// A no-op user config loader for benchmarking.
#[derive(Debug, Default)]
struct NoOpUserConfigLoader;

#[async_trait::async_trait(?Send)]
impl vite_task::loader::UserConfigLoader for NoOpUserConfigLoader {
    async fn load_user_config_file(
        &self,
        _package_path: &AbsolutePath,
    ) -> anyhow::Result<Option<vite_task::config::UserRunConfig>> {
        Ok(None)
    }
}

/// Owned session callbacks for benchmarking.
#[derive(Default)]
struct BenchSessionCallbacks {
    command_handler: NoOpCommandHandler,
    user_config_loader: NoOpUserConfigLoader,
}

impl BenchSessionCallbacks {
    fn as_callbacks(&mut self) -> SessionCallbacks<'_> {
        SessionCallbacks {
            command_handler: &mut self.command_handler,
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
                let envs: FxHashMap<Arc<OsStr>, Arc<OsStr>> = FxHashMap::default();
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
                let envs: FxHashMap<Arc<OsStr>, Arc<OsStr>> = FxHashMap::default();
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
