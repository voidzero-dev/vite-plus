use std::{future::Future, iter::once};

use petgraph::stable_graph::StableGraph;
use vite_error::Error as ViteError;
use vite_task::{
    Error, ExecutionPlan, ExecutionSummary, ResolveCommandResult, ResolvedTask, Workspace,
};

pub async fn test<
    Test: Future<Output = Result<ResolveCommandResult, ViteError>>,
    TestFn: Fn() -> Test,
>(
    resolve_test_command: TestFn,
    workspace: &Workspace,
    args: &Vec<String>,
) -> Result<ExecutionSummary, Error> {
    let ResolveCommandResult { bin_path, envs } =
        resolve_test_command().await.map_err(|e| Error::Anyhow(e.into()))?;
    let wrapped_command =
        || async { Ok(ResolveCommandResult { bin_path: "node".into(), envs: envs.clone() }) };
    let resolved_task = ResolvedTask::resolve_from_builtin(
        workspace,
        wrapped_command,
        "test",
        once(&bin_path).chain(args.iter()),
    )
    .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
