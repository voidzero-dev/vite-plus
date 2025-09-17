use std::{collections::HashMap, future::Future};

use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};

use crate::{
    Error, ResolveCommandResult, Workspace,
    config::ResolvedTask,
    schedule::{ExecutionPlan, ExecutionSummary},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    pub rules: HashMap<String, String>,
}

#[tracing::instrument(skip(resolve_lint_command, workspace))]
pub async fn lint<
    Lint: Future<Output = Result<ResolveCommandResult, Error>>,
    LintFn: Fn() -> Lint,
>(
    resolve_lint_command: LintFn,
    workspace: &mut Workspace,
    args: &Vec<String>,
) -> Result<ExecutionSummary, Error> {
    let resolved_task =
        ResolvedTask::resolve_from_builtin(workspace, resolve_lint_command, "lint", args.iter())
            .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
