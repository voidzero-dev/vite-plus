use std::future::Future;

use petgraph::stable_graph::StableGraph;

use crate::config::ResolvedTask;
use crate::schedule::ExecutionPlan;
use crate::{Error, ResolveCommandResult, Workspace};

#[tracing::instrument(skip(resolve_fmt_command, workspace))]
pub async fn fmt<
    Fmt: Future<Output = Result<ResolveCommandResult, Error>>,
    FmtFn: Fn() -> Fmt,
>(
    resolve_fmt_command: FmtFn,
    workspace: &mut Workspace,
    args: &Vec<String>,
) -> Result<(), Error> {
    let resolved_task =
        ResolvedTask::resolve_from_builtin(workspace, resolve_fmt_command, "fmt", args.iter())
            .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await?;
    Ok(())
}