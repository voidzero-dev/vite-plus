use std::{
    env,
    io::{self, IsTerminal, Write},
    iter,
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal,
};
use petgraph::stable_graph::StableGraph;
use vite_package_manager::package_manager::{PackageManager, PackageManagerType};
use vite_path::AbsolutePathBuf;

use crate::{
    Error, ResolveCommandResult, Workspace,
    config::ResolvedTask,
    schedule::{ExecutionPlan, ExecutionSummary},
};

/// Install command.
///
/// This is the command that will be executed by the `vite-plus install` command.
///
pub struct InstallCommand {
    workspace_root: AbsolutePathBuf,
    ignore_replay: bool,
}

/// Install command builder.
///
/// This is a builder pattern for the `vite-plus install` command.
///
pub struct InstallCommandBuilder {
    workspace_root: AbsolutePathBuf,
    ignore_replay: bool,
}

impl InstallCommand {
    pub const fn builder(workspace_root: AbsolutePathBuf) -> InstallCommandBuilder {
        InstallCommandBuilder::new(workspace_root)
    }

    pub async fn execute(self, args: &Vec<String>) -> Result<ExecutionSummary, Error> {
        // Handle UnrecognizedPackageManager error and let user select a package manager
        let package_manager = match PackageManager::builder(&self.workspace_root).build().await {
            Ok(pm) => pm,
            Err(Error::UnrecognizedPackageManager) => {
                // Prompt user to select a package manager
                let selected_type = prompt_package_manager_selection()?;
                PackageManager::builder(&self.workspace_root)
                    .package_manager_type(selected_type)
                    .build()
                    .await?
            }
            Err(e) => return Err(e),
        };
        let workspace = Workspace::partial_load(self.workspace_root)?;
        let resolve_command = package_manager.resolve_command();
        let resolved_task = ResolvedTask::resolve_from_builtin_with_command_result(
            &workspace,
            "install",
            iter::once("install").chain(args.iter().map(String::as_str)),
            ResolveCommandResult { bin_path: resolve_command.bin_path, envs: resolve_command.envs },
            self.ignore_replay,
            None,
        )?;
        let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
        task_graph.add_node(resolved_task);
        let summary = ExecutionPlan::plan(task_graph, false)?.execute(&workspace).await?;
        workspace.unload().await?;

        Ok(summary)
    }
}

impl InstallCommandBuilder {
    pub const fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root, ignore_replay: false }
    }

    pub const fn ignore_replay(mut self) -> Self {
        self.ignore_replay = true;
        self
    }

    pub fn build(self) -> InstallCommand {
        InstallCommand { workspace_root: self.workspace_root, ignore_replay: self.ignore_replay }
    }
}

/// Common CI environment variables
const CI_ENV_VARS: &[&str] = &[
    "CI",
    "CONTINUOUS_INTEGRATION",
    "GITHUB_ACTIONS",
    "GITLAB_CI",
    "CIRCLECI",
    "TRAVIS",
    "JENKINS_URL",
    "BUILDKITE",
    "DRONE",
    "CODEBUILD_BUILD_ID", // AWS CodeBuild
    "TF_BUILD",           // Azure Pipelines
];

/// Check if running in a CI environment
fn is_ci_environment() -> bool {
    CI_ENV_VARS.iter().any(|key| env::var(key).is_ok())
}

/// Interactive menu for selecting a package manager with keyboard navigation
fn interactive_package_manager_menu() -> Result<PackageManagerType, Error> {
    let options = [
        ("pnpm (recommended)", PackageManagerType::Pnpm),
        ("npm", PackageManagerType::Npm),
        ("yarn", PackageManagerType::Yarn),
    ];

    let mut selected_index = 0;

    // Print header and instructions with proper line breaks
    println!("\n📦 No package manager detected. Please select one:");
    println!(
        "   Use ↑↓ arrows to navigate, Enter to select, 1-{} for quick selection",
        options.len()
    );
    println!("   Press Esc, q, or Ctrl+C to cancel installation\n");

    // Enable raw mode for keyboard input
    terminal::enable_raw_mode()?;

    // Clear the selection area and hide cursor
    execute!(io::stdout(), cursor::Hide)?;

    let result = loop {
        // Display menu with current selection
        for (i, (name, _)) in options.iter().enumerate() {
            execute!(io::stdout(), cursor::MoveToColumn(2))?;

            if i == selected_index {
                // Highlight selected item
                execute!(
                    io::stdout(),
                    SetForegroundColor(Color::Cyan),
                    Print("▶ "),
                    Print(format!("[{}] ", i + 1)),
                    Print(name),
                    ResetColor,
                    Print(" ← ")
                )?;
            } else {
                execute!(
                    io::stdout(),
                    Print("  "),
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("[{}] ", i + 1)),
                    ResetColor,
                    Print(name),
                    Print("   ")
                )?;
            }

            if i < options.len() - 1 {
                execute!(io::stdout(), Print("\n"))?;
            }
        }

        // Move cursor back up for next iteration
        if options.len() > 1 {
            execute!(io::stdout(), cursor::MoveUp((options.len() - 1) as u16))?;
        }

        // Read keyboard input
        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
            match code {
                // Handle Ctrl+C for exit
                KeyCode::Char('c') if modifiers.contains(event::KeyModifiers::CONTROL) => {
                    // Clean up terminal before exiting
                    terminal::disable_raw_mode()?;
                    execute!(
                        io::stdout(),
                        cursor::Show,
                        cursor::MoveDown(options.len() as u16),
                        Print("\n\n"),
                        SetForegroundColor(Color::Yellow),
                        Print("⚠ Installation cancelled by user\n"),
                        ResetColor
                    )?;
                    return Err(Error::UserCancelled);
                }
                KeyCode::Up => {
                    selected_index = selected_index.saturating_sub(1);
                }
                KeyCode::Down => {
                    if selected_index < options.len() - 1 {
                        selected_index += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    break Ok(options[selected_index].1);
                }
                KeyCode::Char('1') => {
                    break Ok(options[0].1);
                }
                KeyCode::Char('2') if options.len() > 1 => {
                    break Ok(options[1].1);
                }
                KeyCode::Char('3') if options.len() > 2 => {
                    break Ok(options[2].1);
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    // Exit on escape/quit
                    terminal::disable_raw_mode()?;
                    execute!(
                        io::stdout(),
                        cursor::Show,
                        cursor::MoveDown(options.len() as u16),
                        Print("\n\n"),
                        SetForegroundColor(Color::Yellow),
                        Print("⚠ Installation cancelled by user\n"),
                        ResetColor
                    )?;
                    return Err(Error::UserCancelled);
                }
                _ => {}
            }
        }
    };

    // Clean up: disable raw mode and show cursor
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), cursor::Show, cursor::MoveDown(options.len() as u16), Print("\n"))?;

    // Print selection confirmation
    if let Ok(pm) = &result {
        let name = match pm {
            PackageManagerType::Pnpm => "pnpm",
            PackageManagerType::Npm => "npm",
            PackageManagerType::Yarn => "yarn",
        };
        println!("\n✓ Selected package manager: {name}\n");
    }

    result
}

/// Prompt the user to select a package manager
fn prompt_package_manager_selection() -> Result<PackageManagerType, Error> {
    // In CI environment, automatically use pnpm without prompting
    if is_ci_environment() {
        println!("CI environment detected. Using default package manager: pnpm");
        return Ok(PackageManagerType::Pnpm);
    }

    // Check if stdin is a TTY (terminal) - if not, use default
    if !io::stdin().is_terminal() {
        println!("Non-interactive environment detected. Using default package manager: pnpm");
        return Ok(PackageManagerType::Pnpm);
    }

    // Try interactive menu first, fall back to simple prompt on error
    match interactive_package_manager_menu() {
        Ok(pm) => Ok(pm),
        Err(err) => {
            match err {
                Error::UserCancelled => Err(err),
                // Fallback to simple text prompt if interactive menu fails
                _ => simple_text_prompt(),
            }
        }
    }
}

/// Simple text-based prompt as fallback
fn simple_text_prompt() -> Result<PackageManagerType, Error> {
    let managers = [
        ("pnpm", PackageManagerType::Pnpm),
        ("npm", PackageManagerType::Npm),
        ("yarn", PackageManagerType::Yarn),
    ];

    println!("\nNo package manager detected. Please select one:");
    println!("────────────────────────────────────────────────");

    for (i, (name, _)) in managers.iter().enumerate() {
        if i == 0 {
            println!("  [{}] {} (recommended)", i + 1, name);
        } else {
            println!("  [{}] {}", i + 1, name);
        }
    }

    print!("\nEnter your choice (1-{}) [default: 1]: ", managers.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let choice = input.trim();
    let index = if choice.is_empty() {
        0 // Default to pnpm
    } else {
        choice
            .parse::<usize>()
            .ok()
            .and_then(|n| if n > 0 && n <= managers.len() { Some(n - 1) } else { None })
            .unwrap_or(0) // Default to pnpm if invalid input
    };

    let (name, selected_type) = &managers[index];
    println!("✓ Selected package manager: {name}\n");

    Ok(*selected_type)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;

    /// Helper struct to safely manage environment variables in tests
    /// This struct ensures that environment variables are properly restored
    /// after the test completes, even if the test panics.
    struct EnvGuard {
        key: String,
        original_value: Option<String>,
    }

    impl EnvGuard {
        fn new(key: &str, value: &str) -> Self {
            let original_value = env::var(key).ok();
            // SAFETY: This is only used in tests which are run serially,
            // preventing data races on environment variables
            unsafe {
                env::set_var(key, value);
            }
            Self { key: key.to_string(), original_value }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: This is only used in tests which are run serially,
            // preventing data races on environment variables
            unsafe {
                match &self.original_value {
                    Some(value) => env::set_var(&self.key, value),
                    None => env::remove_var(&self.key),
                }
            }
        }
    }

    #[test]
    fn test_install_command_builder_build() {
        let workspace_root = AbsolutePathBuf::new(PathBuf::from(if cfg!(windows) {
            "C:\\test\\workspace"
        } else {
            "/test/workspace"
        }))
        .unwrap();
        let command = InstallCommandBuilder::new(workspace_root.clone()).build();

        assert_eq!(command.workspace_root, workspace_root);
    }

    #[ignore = "skip this test for auto run, should be run manually, because it will prompt for user selection"]
    #[tokio::test]
    async fn test_install_command_with_package_json_without_package_manager() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create a minimal package.json
        let package_json = r#"{
            "name": "test-package",
            "version": "1.0.0"
        }"#;
        fs::write(workspace_root.join("package.json"), package_json).unwrap();

        let command = InstallCommandBuilder::new(workspace_root).build();
        assert!(command.execute(&vec![]).await.is_ok());
    }

    #[tokio::test]
    #[cfg(not(windows))] // FIXME
    async fn test_install_command_with_package_json_with_package_manager() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create a minimal package.json
        let package_json = r#"{
            "name": "test-package",
            "version": "1.0.0",
            "packageManager": "pnpm@10.15.0"
        }"#;
        fs::write(workspace_root.join("package.json"), package_json).unwrap();

        let command = InstallCommandBuilder::new(workspace_root).build();
        let result = command.execute(&vec![]).await;
        println!("result: {:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_install_command_execute_with_invalid_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = AbsolutePathBuf::new(temp_dir.path().join("nonexistent")).unwrap();

        let command = InstallCommandBuilder::new(workspace_root).build();
        let args = vec![];

        let result = command.execute(&args).await;
        assert!(matches!(result.unwrap_err(), Error::PackageJsonNotFound(_)));
    }

    /// Test that in CI environment, we will use pnpm without prompting
    #[test]
    #[serial] // Run serially to avoid race conditions with environment variables
    fn test_prompt_package_manager_in_ci() {
        // Use EnvGuard to safely manage the CI environment variable
        let _guard = EnvGuard::new("CI", "true");

        // Should return pnpm without prompting
        let result = prompt_package_manager_selection();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PackageManagerType::Pnpm);

        // EnvGuard will automatically restore the original value when dropped
    }
}
