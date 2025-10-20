use std::{collections::HashMap, future::Future, iter::once};

use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};
use vite_error::Error as ViteError;
use vite_task::{
    Error, ExecutionPlan, ExecutionSummary, ResolveCommandResult, ResolvedTask, Workspace,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    pub rules: HashMap<String, String>,
}

#[tracing::instrument(skip(resolve_lint_command, workspace))]
pub async fn lint<
    Lint: Future<Output = Result<ResolveCommandResult, ViteError>>,
    LintFn: Fn() -> Lint,
>(
    resolve_lint_command: LintFn,
    workspace: &Workspace,
    args: &Vec<String>,
) -> Result<ExecutionSummary, Error> {
    let ResolveCommandResult { bin_path, envs } =
        resolve_lint_command().await.map_err(|e| Error::Anyhow(e.into()))?;
    let wrapped_command =
        || async { Ok(ResolveCommandResult { bin_path: "node".into(), envs: envs.clone() }) };
    let resolved_task = ResolvedTask::resolve_from_builtin(
        workspace,
        wrapped_command,
        "lint",
        once(&bin_path).chain(args.iter()),
    )
    .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
