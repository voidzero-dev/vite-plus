use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::{env, iter};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal,
};
use petgraph::stable_graph::StableGraph;

use crate::config::ResolvedTask;
use crate::schedule::ExecutionPlan;
use crate::{Error, ResolveCommandResult, Workspace};
use vite_package_manager::package_manager::{
    PackageManagerType, detect_package_manager, detect_package_manager_with_default,
};

/// Install command.
///
/// This is the command that will be executed by the `vite-plus install` command.
///
pub struct InstallCommand {
    workspace_root: PathBuf,
    force_refresh_cached: Option<bool>,
    replay_cached_outputs: Option<bool>,
}

/// Install command builder.
///
/// This is a builder pattern for the `vite-plus install` command.
///
pub struct InstallCommandBuilder {
    workspace_root: PathBuf,
    /// Whether to force run the install command.
    force_refresh_cached: Option<bool>,
    /// Whether to replay cache outputs.
    replay_cached_outputs: Option<bool>,
}

impl InstallCommand {
    pub fn builder(workspace_root: impl AsRef<Path>) -> InstallCommandBuilder {
        InstallCommandBuilder::new(workspace_root)
    }

    pub async fn execute(self, args: &Vec<String>) -> Result<(), Error> {
        // Handle UnrecognizedPackageManager error and let user select a package manager
        let package_manager = match detect_package_manager(&self.workspace_root).await {
            Ok(pm) => pm,
            Err(Error::UnrecognizedPackageManager) => {
                // Prompt user to select a package manager
                let selected_type = prompt_package_manager_selection()?;
                detect_package_manager_with_default(&self.workspace_root, Some(selected_type))
                    .await?
            }
            Err(e) => return Err(e),
        };
        let mut workspace = Workspace::partial_load(self.workspace_root.clone())?;
        let bin_path = package_manager.bin_name.clone();
        let envs = HashMap::from([(
            "PATH".to_string(),
            format_path_env(package_manager.get_bin_prefix()),
        )]);
        let resolved_task = ResolvedTask::resolve_from_built_in_with_comment_result(
            &workspace,
            "install",
            iter::once("install").chain(args.iter().map(|arg| arg.as_str())),
            ResolveCommandResult { bin_path, envs },
            self.force_refresh_cached,
            self.replay_cached_outputs,
        )?;
        let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
        task_graph.add_node(resolved_task);
        ExecutionPlan::plan(task_graph, false)?.execute(&mut workspace).await?;
        workspace.unload().await?;

        Ok(())
    }
}

impl InstallCommandBuilder {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().into(),
            force_refresh_cached: None,
            replay_cached_outputs: None,
        }
    }

    pub fn force_run(mut self) -> Self {
        self.force_refresh_cached = Some(true);
        self
    }

    pub fn disable_replay_cached_outputs(mut self) -> Self {
        self.replay_cached_outputs = Some(false);
        self
    }

    pub fn build(self) -> InstallCommand {
        InstallCommand {
            workspace_root: self.workspace_root,
            force_refresh_cached: self.force_refresh_cached,
            replay_cached_outputs: self.replay_cached_outputs,
        }
    }
}

fn format_path_env(bin_prefix: impl AsRef<Path>) -> String {
    let mut paths = env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
    paths.insert(0, bin_prefix.as_ref().to_path_buf());
    env::join_paths(paths).unwrap().to_string_lossy().to_string()
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
    let options = vec![
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
    terminal::enable_raw_mode().map_err(|e| Error::Io(e))?;

    // Clear the selection area and hide cursor
    execute!(io::stdout(), cursor::Hide).map_err(|e| Error::Io(e))?;

    let result = loop {
        // Display menu with current selection
        for (i, (name, _)) in options.iter().enumerate() {
            execute!(io::stdout(), cursor::MoveToColumn(2)).map_err(|e| Error::Io(e))?;

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
                )
                .map_err(|e| Error::Io(e))?;
            } else {
                execute!(
                    io::stdout(),
                    Print("  "),
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("[{}] ", i + 1)),
                    ResetColor,
                    Print(name),
                    Print("   ")
                )
                .map_err(|e| Error::Io(e))?;
            }

            if i < options.len() - 1 {
                execute!(io::stdout(), Print("\n")).map_err(|e| Error::Io(e))?;
            }
        }

        // Move cursor back up for next iteration
        if options.len() > 1 {
            execute!(io::stdout(), cursor::MoveUp((options.len() - 1) as u16))
                .map_err(|e| Error::Io(e))?;
        }

        // Read keyboard input
        if let Event::Key(KeyEvent { code, modifiers, .. }) =
            event::read().map_err(|e| Error::Io(e))?
        {
            match code {
                // Handle Ctrl+C for exit
                KeyCode::Char('c') if modifiers.contains(event::KeyModifiers::CONTROL) => {
                    // Clean up terminal before exiting
                    terminal::disable_raw_mode().ok();
                    execute!(
                        io::stdout(),
                        cursor::Show,
                        cursor::MoveDown(options.len() as u16),
                        Print("\n\n"),
                        SetForegroundColor(Color::Yellow),
                        Print("⚠ Installation cancelled by user\n"),
                        ResetColor
                    )
                    .ok();
                    std::process::exit(130); // Standard exit code for Ctrl+C
                }
                KeyCode::Up => {
                    if selected_index > 0 {
                        selected_index -= 1;
                    }
                }
                KeyCode::Down => {
                    if selected_index < options.len() - 1 {
                        selected_index += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    break Ok(options[selected_index].1.clone());
                }
                KeyCode::Char('1') => {
                    break Ok(options[0].1.clone());
                }
                KeyCode::Char('2') if options.len() > 1 => {
                    break Ok(options[1].1.clone());
                }
                KeyCode::Char('3') if options.len() > 2 => {
                    break Ok(options[2].1.clone());
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    // Exit on escape/quit
                    terminal::disable_raw_mode().ok();
                    execute!(
                        io::stdout(),
                        cursor::Show,
                        cursor::MoveDown(options.len() as u16),
                        Print("\n\n"),
                        SetForegroundColor(Color::Yellow),
                        Print("⚠ Installation cancelled by user\n"),
                        ResetColor
                    )
                    .ok();
                    std::process::exit(130); // Standard exit code for user cancellation
                }
                _ => {}
            }
        }
    };

    // Clean up: disable raw mode and show cursor
    terminal::disable_raw_mode().map_err(|e| Error::Io(e))?;
    execute!(io::stdout(), cursor::Show, cursor::MoveDown(options.len() as u16), Print("\n"))
        .map_err(|e| Error::Io(e))?;

    // Print selection confirmation
    match &result {
        Ok(pm) => {
            let name = match pm {
                PackageManagerType::Pnpm => "pnpm",
                PackageManagerType::Npm => "npm",
                PackageManagerType::Yarn => "yarn",
            };
            println!("\n✓ Selected package manager: {}\n", name);
        }
        Err(_) => {}
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
    if !atty::is(atty::Stream::Stdin) {
        println!("Non-interactive environment detected. Using default package manager: pnpm");
        return Ok(PackageManagerType::Pnpm);
    }

    // Try interactive menu first, fall back to simple prompt on error
    match interactive_package_manager_menu() {
        Ok(pm) => Ok(pm),
        Err(_) => {
            // Fallback to simple text prompt if interactive menu fails
            simple_text_prompt()
        }
    }
}

/// Simple text-based prompt as fallback
fn simple_text_prompt() -> Result<PackageManagerType, Error> {
    let managers = vec![
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
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| Error::Io(e))?;

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
    println!("✓ Selected package manager: {}\n", name);

    Ok(selected_type.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_install_command_builder_new() {
        let workspace_root = PathBuf::from("/test/workspace");
        let builder = InstallCommandBuilder::new(&workspace_root);

        assert_eq!(builder.workspace_root, workspace_root);
        assert_eq!(builder.force_refresh_cached, None);
        assert_eq!(builder.replay_cached_outputs, None);
    }

    #[test]
    fn test_install_command_builder_set_force_run() {
        let builder = InstallCommandBuilder::new("/test/workspace").force_run();

        assert_eq!(builder.force_refresh_cached, Some(true));
    }

    #[test]
    fn test_install_command_builder_set_replay_cache_outputs() {
        let builder = InstallCommandBuilder::new("/test/workspace").disable_replay_cached_outputs();

        assert_eq!(builder.replay_cached_outputs, Some(false));
    }

    #[test]
    fn test_install_command_builder_build() {
        let workspace_root = PathBuf::from("/test/workspace");
        let command = InstallCommandBuilder::new(&workspace_root)
            .force_run()
            .disable_replay_cached_outputs()
            .build();

        assert_eq!(command.workspace_root, workspace_root);
        assert_eq!(command.force_refresh_cached, Some(true));
        assert_eq!(command.replay_cached_outputs, Some(false));
    }

    #[test]
    fn test_install_command_builder_chain() {
        // Test that builder methods can be chained
        let command = InstallCommandBuilder::new("/test/workspace")
            .force_run()
            .disable_replay_cached_outputs()
            .build();

        assert_eq!(command.force_refresh_cached, Some(true));
        assert_eq!(command.replay_cached_outputs, Some(false));
    }

    // skip this test for auto run, should be run manually, because it will prompt for user selection
    #[ignore]
    #[tokio::test]
    async fn test_install_command_execute_with_invalid_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path().join("nonexistent");

        let command = InstallCommandBuilder::new(&workspace_root).build();
        let args = vec![];

        let result = command.execute(&args).await;
        // When no package.json exists and no package manager is specified,
        // it will prompt for user selection (in tests this will error as stdin is not available)
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_command_with_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create a minimal package.json
        let package_json = r#"{
            "name": "test-package",
            "version": "1.0.0",
            "packageManager": "npm@10.0.0"
        }"#;
        fs::write(workspace_root.join("package.json"), package_json).unwrap();

        // Create an empty vite-task.json
        let vite_task_json = r#"{
            "tasks": {}
        }"#;
        fs::write(workspace_root.join("vite-task.json"), vite_task_json).unwrap();

        let command = InstallCommandBuilder::new(workspace_root).build();

        // Note: This test will likely fail in CI since it tries to actually run npm install
        // In a real test environment, you'd want to mock the package manager execution
        // For now, we just verify the command can be constructed
        assert_eq!(command.workspace_root, workspace_root);

        // execute install command successfully
        assert!(command.execute(&vec![]).await.is_ok());
    }

    /// Test that in CI environment, we will use pnpm without prompting
    #[test]
    fn test_prompt_package_manager_in_ci() {
        let has_ci_env = env::var("CI").is_ok();
        if !has_ci_env {
            // Set CI environment
            unsafe {
                env::set_var("CI", "true");
            }
        }

        // Should return pnpm without prompting
        let result = prompt_package_manager_selection();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PackageManagerType::Pnpm);

        if !has_ci_env {
            // Clean up
            unsafe {
                env::remove_var("CI");
            }
        }
    }
}
