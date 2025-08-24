use std::collections::BTreeMap;
use std::fmt::Display;

use bincode::{Decode, Encode};
use brush_parser::{
    Parser, ParserOptions,
    ast::{
        AndOr, Assignment, AssignmentName, AssignmentValue, Command, CommandPrefix,
        CommandPrefixOrSuffixItem, CommandSuffix, CompoundListItem, Pipeline, Program,
        SeparatorOperator, SimpleCommand, Word,
    },
    unquote_str,
};
use diff::Diff;
use serde::Serialize;

use crate::str::Str;

/// "FOO=BAR program arg1 arg2"
#[derive(Encode, Decode, Serialize, Debug, PartialEq, Eq, Diff, Clone)]
#[diff(attr(#[derive(Debug)]))]
pub struct TaskParsedCommand {
    pub envs: BTreeMap<Str, Str>,
    pub program: Str,
    pub args: Vec<Str>,
}

impl Display for TaskParsedCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // BTreeMap ensures stable iteration order
        for (name, value) in &self.envs {
            Display::fmt(
                &format_args!("{}={} ", name, shell_escape::escape(value.as_str().into())),
                f,
            )?;
        }
        Display::fmt(&shell_escape::escape(self.program.as_str().into()), f)?;
        for arg in &self.args {
            Display::fmt(" ", f)?;
            Display::fmt(&shell_escape::escape(arg.as_str().into()), f)?;
        }

        Ok(())
    }
}

fn unquote(word: &Word) -> String {
    let Word { value } = word;
    unquote_str(value.as_str())
}

fn pipeline_to_command(pipeline: &Pipeline) -> Option<TaskParsedCommand> {
    let Pipeline { timed: None, bang: false, seq } = pipeline else {
        return None;
    };
    let [Command::Simple(simple_command)] = seq.as_slice() else {
        return None;
    };
    let SimpleCommand { prefix, word_or_name: Some(program), suffix } = simple_command else {
        return None;
    };
    let mut envs = BTreeMap::<Str, Str>::new();
    if let Some(prefix) = prefix {
        let CommandPrefix(items) = prefix;
        for item in items {
            let CommandPrefixOrSuffixItem::AssignmentWord(
                Assignment { name, value, append: false },
                _,
            ) = item
            else {
                return None;
            };
            let AssignmentName::VariableName(name) = name else {
                return None;
            };
            let AssignmentValue::Scalar(value) = value else {
                return None;
            };
            envs.insert(name.as_str().into(), unquote(value).into());
        }
    }
    let mut args = Vec::<Str>::new();
    if let Some(CommandSuffix(suffix_items)) = suffix {
        for suffix_item in suffix_items {
            let CommandPrefixOrSuffixItem::Word(word) = suffix_item else {
                return None;
            };
            args.push(unquote(word).into());
        }
    }
    Some(TaskParsedCommand { envs, program: unquote(program).into(), args })
}

pub fn try_parse_as_and_list(cmd: &str) -> Option<Vec<TaskParsedCommand>> {
    let mut parser = Parser::new(
        cmd.as_bytes(),
        &ParserOptions {
            enable_extended_globbing: false,
            posix_mode: true,
            sh_mode: true,
            tilde_expansion: false,
        },
        &Default::default(),
    );
    let Program { complete_commands } = parser.parse_program().ok()?;
    let [compound_list] = complete_commands.as_slice() else {
        return None;
    };
    let [CompoundListItem(and_or_list, SeparatorOperator::Sequence)] = compound_list.0.as_slice()
    else {
        return None;
    };

    let mut commands = Vec::<TaskParsedCommand>::new();
    commands.push(pipeline_to_command(&and_or_list.first)?);
    for and_or in &and_or_list.additional {
        let AndOr::And(pipeline) = and_or else {
            return None;
        };
        commands.push(pipeline_to_command(pipeline)?);
    }
    Some(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        assert_eq!(
            try_parse_as_and_list(r#"hello world && FOO="BE\"R" program "arg1" "arg\"2" && zzz"#),
            Some(vec![
                TaskParsedCommand {
                    envs: [].into(),
                    program: "hello".into(),
                    args: vec!["world".into()],
                },
                TaskParsedCommand {
                    envs: [("FOO".into(), "BE\"R".into())].into(),
                    program: "program".into(),
                    args: vec!["arg1".into(), "arg\"2".into()],
                },
                TaskParsedCommand { envs: [].into(), program: "zzz".into(), args: vec![] }
            ])
        );
    }

    #[test]
    fn test_task_parsed_command_stable_env_ordering() {
        // Test that environment variables maintain stable ordering
        let cmd = TaskParsedCommand {
            envs: [
                ("ZEBRA".into(), "last".into()),
                ("ALPHA".into(), "first".into()),
                ("MIDDLE".into(), "middle".into()),
            ]
            .into(),
            program: "test".into(),
            args: vec![],
        };

        // Convert to string multiple times and verify it's always the same
        let str1 = cmd.to_string();
        let str2 = cmd.to_string();
        let str3 = cmd.to_string();

        assert_eq!(str1, str2);
        assert_eq!(str2, str3);

        // Verify the order is alphabetical (BTreeMap sorts by key)
        assert!(str1.starts_with("ALPHA=first MIDDLE=middle ZEBRA=last"));
    }

    #[test]
    fn test_task_parsed_command_serialization_stability() {
        use bincode::{decode_from_slice, encode_to_vec};

        // Create a command with multiple environment variables
        let cmd = TaskParsedCommand {
            envs: [
                ("VAR_C".into(), "value_c".into()),
                ("VAR_A".into(), "value_a".into()),
                ("VAR_B".into(), "value_b".into()),
            ]
            .into(),
            program: "program".into(),
            args: vec!["arg1".into(), "arg2".into()],
        };

        // Serialize multiple times
        let config = bincode::config::standard();
        let bytes1 = encode_to_vec(&cmd, config).unwrap();
        let bytes2 = encode_to_vec(&cmd, config).unwrap();

        // Verify serialization is stable
        assert_eq!(bytes1, bytes2);

        // Verify deserialization works and maintains order
        let (decoded, _): (TaskParsedCommand, _) = decode_from_slice(&bytes1, config).unwrap();
        assert_eq!(decoded, cmd);

        // Verify the decoded command still has stable string representation
        assert_eq!(decoded.to_string(), cmd.to_string());
    }
}
