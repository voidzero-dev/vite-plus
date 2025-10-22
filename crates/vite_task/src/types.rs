use std::collections::HashMap;

/// Result from resolving a command
pub struct ResolveCommandResult {
    pub bin_path: String,
    pub envs: HashMap<String, String>,
}
