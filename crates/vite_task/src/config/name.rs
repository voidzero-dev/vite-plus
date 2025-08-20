use std::fmt::Display;

use bincode::enc::write;

use crate::str::Str;

/// For displaying and filtering tasks.
///
/// Not suitable for uniquely identifying tasks as package_name isn't unique (may be duplicated and empty).
/// See `TaskId` for the unique identifier of a task.
#[derive(Debug, Clone)]
pub struct TaskName {
    /// The name of the script containing this task.
    /// For example, in script `"build": "echo A && echo B"`,
    /// Both task `echo A` and task `echo B` will have `task_group_name` = "build".
    pub task_group_name: Str,

    /// The name of the package where this task is defined. Can be empty when field `name` is not defined in a package.
    pub package_name: Str,

    /// The index of the subcommand in a parsed command (`echo A && echo B`).
    /// `None` if the task is a main task, which is the last subcommand or the only subcommand in a script.
    /// Only the main command can be matched agaist a user task request.
    /// Non-main commands can only be included in the execution graph as main command's (in)direct dependencies.
    pub subcommand_index: Option<usize>,
}

impl Display for TaskName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.package_name.is_empty() {
            write!(f, "{}#", self.package_name)?;
        }
        write!(f, "{}", self.task_group_name)?;
        if let Some(subcommand_index) = self.subcommand_index {
            write!(f, "(subcommand {})", subcommand_index)?;
        }
        Ok(())
    }
}
