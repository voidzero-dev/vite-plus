use std::future::Future;
use std::iter;

use petgraph::stable_graph::StableGraph;

use crate::config::ResolvedTask;
use crate::schedule::ExecutionPlan;
use crate::{Error, ResolveCommandResult, Workspace};

pub async fn test<
    Test: Future<Output = Result<ResolveCommandResult, Error>>,
    TestFn: Fn() -> Test,
>(
    resolve_test_command: TestFn,
    workspace: &mut Workspace,
    args: &Vec<String>,
) -> Result<(), Error> {
    let resolved_task = ResolvedTask::resolve_from_built_in(
        workspace,
        resolve_test_command,
        "test",
        iter::once("test").chain(args.iter().map(|arg| arg.as_str())),
    )
    .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await?;
    Ok(())
}
