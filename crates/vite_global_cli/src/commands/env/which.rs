//! Which command implementation.
//!
//! Shows the path to the tool binary that would be executed.

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use super::config::resolve_version;
use crate::error::Error;

/// Supported tools
const SUPPORTED_TOOLS: &[&str] = &["node", "npm", "npx"];

/// Execute the which command.
pub async fn execute(cwd: AbsolutePathBuf, tool: &str) -> Result<ExitStatus, Error> {
    // Validate tool name
    if !SUPPORTED_TOOLS.contains(&tool) {
        eprintln!("vp: Unknown tool '{tool}'");
        eprintln!("Supported tools: {}", SUPPORTED_TOOLS.join(", "));
        return Ok(exit_status(1));
    }

    // Resolve version for current directory
    let resolution = resolve_version(&cwd).await?;

    // Get the tool path
    let home_dir = vite_shared::get_vite_plus_home()?
        .join("js_runtime")
        .join("node")
        .join(&resolution.version);

    #[cfg(windows)]
    let tool_path = if tool == "node" {
        home_dir.join("node.exe")
    } else {
        home_dir.join(format!("{tool}.cmd"))
    };

    #[cfg(not(windows))]
    let tool_path = home_dir.join("bin").join(tool);

    // Check if the tool exists
    if !tokio::fs::try_exists(&tool_path).await.unwrap_or(false) {
        eprintln!("vp: {} not found at {}", tool, tool_path.as_path().display());
        eprintln!("Node.js {} may not be installed yet.", resolution.version);
        eprintln!("Run 'node -v' to trigger installation.");
        return Ok(exit_status(1));
    }

    println!("{}", tool_path.as_path().display());

    Ok(ExitStatus::default())
}

/// Create an exit status with the given code.
fn exit_status(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
}
