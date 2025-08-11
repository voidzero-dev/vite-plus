use std::future::Future;

use petgraph::stable_graph::StableGraph;

use crate::config::ResolvedTask;
use crate::schedule::ExecutionPlan;
use crate::{Error, ResolveCommandResult, Workspace};

pub async fn lint<
    Lint: Future<Output = Result<ResolveCommandResult, Error>>,
    LintFn: Fn() -> Lint,
>(
    resolve_lint_command: LintFn,
    workspace: &mut Workspace,
    args: &Vec<String>,
) -> Result<(), Error> {
    let resolved_task =
        ResolvedTask::resolve_from_built_in(workspace, resolve_lint_command, "lint", args).await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await?;
    Ok(())
}
