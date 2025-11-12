use std::{future::Future, iter::once};

use petgraph::stable_graph::StableGraph;
use vite_error::Error as ViteError;
use vite_task::{
    Error, ExecutionPlan, ExecutionSummary, ResolveCommandResult, ResolvedTask, Workspace,
};

#[tracing::instrument(skip(resolve_lib_command, workspace))]
pub async fn lib<
    Lib: Future<Output = Result<ResolveCommandResult, ViteError>>,
    LibFn: Fn() -> Lib,
>(
    resolve_lib_command: LibFn,
    workspace: &Workspace,
    args: &Vec<String>,
) -> Result<ExecutionSummary, Error> {
    let ResolveCommandResult { bin_path, envs } =
        resolve_lib_command().await.map_err(|e| Error::Anyhow(e.into()))?;
    let wrapped_command = || async {
        Ok(ResolveCommandResult {
            bin_path: (if cfg!(windows) { "node.exe" } else { "node" }).into(),
            envs: envs.clone(),
        })
    };
    let resolved_task = ResolvedTask::resolve_from_builtin(
        workspace,
        wrapped_command,
        "lib",
        once(&bin_path).chain(args.iter()),
    )
    .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
