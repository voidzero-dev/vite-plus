use std::{future::Future, iter};

use petgraph::stable_graph::StableGraph;

use crate::{
    Error, ResolveCommandResult, Workspace,
    config::ResolvedTask,
    schedule::{ExecutionPlan, ExecutionSummary},
};

pub async fn test<
    Test: Future<Output = Result<ResolveCommandResult, Error>>,
    TestFn: Fn() -> Test,
>(
    resolve_test_command: TestFn,
    workspace: &mut Workspace,
    args: &Vec<String>,
) -> Result<ExecutionSummary, Error> {
    let resolved_task = ResolvedTask::resolve_from_builtin(
        workspace,
        resolve_test_command,
        "test",
        iter::once("test").chain(args.iter().map(std::string::String::as_str)),
    )
    .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
