use std::{future::Future, process::ExitStatus};

use petgraph::stable_graph::StableGraph;

use crate::{
    Error, ResolveCommandResult, Workspace, config::ResolvedTask, schedule::ExecutionPlan,
};

#[tracing::instrument(skip(resolve_lint_command, workspace))]
pub async fn lint<
    Lint: Future<Output = Result<ResolveCommandResult, Error>>,
    LintFn: Fn() -> Lint,
>(
    resolve_lint_command: LintFn,
    workspace: &mut Workspace,
    args: &Vec<String>,
) -> Result<Option<ExitStatus>, Error> {
    let resolved_task =
        ResolvedTask::resolve_from_builtin(workspace, resolve_lint_command, "lint", args.iter())
            .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
