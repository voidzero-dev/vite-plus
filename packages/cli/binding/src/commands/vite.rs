use std::future::Future;

use petgraph::stable_graph::StableGraph;
use vite_error::Error as ViteError;
use vite_task::{
    Error, ExecutionPlan, ExecutionSummary, ResolveCommandResult, ResolvedTask, Workspace,
};

pub async fn vite<
    Vite: Future<Output = Result<ResolveCommandResult, ViteError>>,
    ViteFn: Fn() -> Vite,
>(
    arg_forward: &str,
    resolve_vite_command: ViteFn,
    workspace: &Workspace,
    args: &Vec<String>,
) -> Result<ExecutionSummary, Error> {
    let ResolveCommandResult { bin_path, envs } =
        resolve_vite_command().await.map_err(|e| Error::Anyhow(e.into()))?;
    let wrapped_command =
        || async { Ok(ResolveCommandResult { bin_path: "node".into(), envs: envs.clone() }) };
    let resolved_task = ResolvedTask::resolve_from_builtin(
        workspace,
        wrapped_command,
        arg_forward,
        [bin_path.as_str(), arg_forward]
            .into_iter()
            .chain(args.iter().map(std::string::String::as_str)),
    )
    .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
