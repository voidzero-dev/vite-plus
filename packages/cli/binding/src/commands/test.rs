use std::future::Future;

use petgraph::stable_graph::StableGraph;
use vite_error::Error;
use vite_task::{ExecutionPlan, ExecutionSummary, ResolveCommandResult, ResolvedTask, Workspace};

pub async fn test<
    Test: Future<Output = Result<ResolveCommandResult, Error>>,
    TestFn: Fn() -> Test,
>(
    resolve_test_command: TestFn,
    workspace: &Workspace,
    args: &Vec<String>,
) -> Result<ExecutionSummary, Error> {
    let resolved_task = ResolvedTask::resolve_from_builtin(
        workspace,
        resolve_test_command,
        "test",
        args.iter().map(std::string::String::as_str),
    )
    .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
