use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandResolution {
    Run(ResolvedCommand),
    Noop,
    InvalidArgument(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedCommand {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) env: BTreeMap<String, String>,
    pub(crate) pre_run: Vec<PreRunAction>,
}

impl ResolvedCommand {
    pub(crate) fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
            pre_run: Vec::new(),
        }
    }
}

impl From<CommandBuilder> for CommandResolution {
    fn from(builder: CommandBuilder) -> Self {
        Self::Run(builder.build())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreRunAction {
    CreateDir { path: String },
}

#[derive(Debug, Clone)]
pub(crate) struct CommandBuilder {
    command: ResolvedCommand,
}

impl CommandBuilder {
    pub(crate) fn new(program: impl Into<String>) -> Self {
        Self { command: ResolvedCommand::new(program) }
    }

    pub(crate) fn arg(&mut self, arg: impl ToString) -> &mut Self {
        self.command.args.push(arg.to_string());
        self
    }

    pub(crate) fn arg_if(&mut self, arg: &str, condition: bool) -> &mut Self {
        if condition {
            self.arg(arg);
        }
        self
    }

    pub(crate) fn option<T>(&mut self, flag: &str, value: Option<T>) -> &mut Self
    where
        T: ToString,
    {
        if let Some(value) = value {
            self.arg(flag);
            self.arg(value.to_string());
        }
        self
    }

    pub(crate) fn repeated<'a, T, I>(&mut self, flag: &str, values: I) -> &mut Self
    where
        T: ToString + 'a,
        I: IntoIterator<Item = &'a T>,
    {
        for value in values {
            self.arg(flag);
            self.arg(value.to_string());
        }
        self
    }

    pub(crate) fn extend<'a, T, I>(&mut self, values: I) -> &mut Self
    where
        T: ToString + 'a,
        I: IntoIterator<Item = &'a T>,
    {
        for value in values {
            self.arg(value.to_string());
        }
        self
    }

    pub(crate) fn create_dir(&mut self, path: impl ToString) -> &mut Self {
        self.command.pre_run.push(PreRunAction::CreateDir { path: path.to_string() });
        self
    }

    pub(crate) fn build(self) -> ResolvedCommand {
        self.command
    }
}
