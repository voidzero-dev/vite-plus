use std::{collections::HashMap, future::Future};

use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Error, ResolveCommandResult, Workspace,
    config::ResolvedTask,
    schedule::{ExecutionPlan, ExecutionSummary},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FmtConfig {
    pub rules: HashMap<String, Value>,
}

#[tracing::instrument(skip(resolve_fmt_command, workspace))]
pub async fn fmt<Fmt: Future<Output = Result<ResolveCommandResult, Error>>, FmtFn: Fn() -> Fmt>(
    resolve_fmt_command: FmtFn,
    workspace: &mut Workspace,
    args: &Vec<String>,
) -> Result<ExecutionSummary, Error> {
    let resolved_task =
        ResolvedTask::resolve_from_builtin(workspace, resolve_fmt_command, "fmt", args.iter())
            .await?;
    let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
    task_graph.add_node(resolved_task);
    ExecutionPlan::plan(task_graph, false)?.execute(workspace).await
}
