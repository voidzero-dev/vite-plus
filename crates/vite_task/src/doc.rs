use std::future::Future;

use petgraph::stable_graph::StableGraph;

use crate::{
    Error, ResolveCommandResult, Workspace,
    config::ResolvedTask,
    schedule::{ExecutionPlan, ExecutionSummary},
};

pub async fn doc<Doc: Future<Output = Result<ResolveCommandResult, Error>>, DocFn: Fn() -> Doc>(
    resolve_doc_command: DocFn,
    workspace: &Workspace,
    args: &Vec<String>,
) -> Result<ExecutionSummary, Error> {
    let resolved_task =
        ResolvedTask::resolve_from_builtin(workspace, resolve_doc_command, "doc", args.iter())
            .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
