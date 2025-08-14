use std::future::Future;
use std::iter;

use petgraph::stable_graph::StableGraph;

use crate::config::ResolvedTask;
use crate::schedule::ExecutionPlan;
use crate::{Error, ResolveCommandResult, Workspace};

pub async fn build<
    Build: Future<Output = Result<ResolveCommandResult, Error>>,
    BuildFn: Fn() -> Build,
>(
    resolve_build_command: BuildFn,
    workspace: &mut Workspace,
    args: &Vec<String>,
) -> Result<(), Error> {
    let resolved_task = ResolvedTask::resolve_from_built_in(
        workspace,
        resolve_build_command,
        "build",
        iter::once("build").chain(args.iter().map(|arg| arg.as_str())),
    )
    .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await?;
    Ok(())
}
