use std::{future::Future, iter};

use petgraph::stable_graph::StableGraph;
use vite_error::Error;
use vite_task::{ExecutionPlan, ExecutionSummary, ResolveCommandResult, ResolvedTask, Workspace};

pub async fn vite<
    Vite: Future<Output = Result<ResolveCommandResult, Error>>,
    ViteFn: Fn() -> Vite,
>(
    arg_forward: &str,
    resolve_vite_command: ViteFn,
    workspace: &Workspace,
    args: &Vec<String>,
) -> Result<ExecutionSummary, Error> {
    let resolved_task = ResolvedTask::resolve_from_builtin(
        workspace,
        resolve_vite_command,
        arg_forward,
        iter::once(arg_forward).chain(args.iter().map(std::string::String::as_str)),
    )
    .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
