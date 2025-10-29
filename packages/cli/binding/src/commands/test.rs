use std::future::Future;

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
    let wrapped_command =
        || async { resolve_test_command().await.map_err(|e| Error::Anyhow(e.into())) };
    let resolved_task = ResolvedTask::resolve_from_builtin(
        workspace,
        wrapped_command,
        "test",
        args.iter().map(std::string::String::as_str),
    )
    .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
