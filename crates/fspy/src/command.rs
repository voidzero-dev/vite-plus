use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process::Stdio,
};

#[cfg(unix)]
use fspy_shared_unix::exec::Exec;
use tokio::process::Command as TokioCommand;

use crate::{
    TrackedChild,
    os_impl::{self, spawn_impl},
};

#[derive(Debug)]
pub struct Command {
    pub(crate) program: OsString,
    pub(crate) args: Vec<OsString>,
    pub(crate) envs: HashMap<OsString, OsString>,
    pub(crate) cwd: Option<PathBuf>,
    #[cfg(unix)]
    pub(crate) arg0: Option<OsString>,

    pub(crate) stderr: Option<Stdio>,
    pub(crate) stdout: Option<Stdio>,
    pub(crate) stdin: Option<Stdio>,

    pub(crate) spy_inner: os_impl::SpyInner,
}

impl Command {
    #[cfg(unix)]
    pub fn get_exec(&self) -> Exec {
        use std::{
            iter::once,
            os::unix::ffi::{OsStrExt, OsStringExt},
        };

        use bstr::{BString, ByteSlice as _};
        let arg0 =
            BString::from(self.arg0.clone().unwrap_or_else(|| self.program.clone()).into_vec());
        Exec {
            program: self.program.as_bytes().into(),
            args: once(arg0)
                .chain(self.args.iter().map(|arg| arg.as_bytes().as_bstr().to_owned()))
                .collect(),
            envs: self
                .envs
                .iter()
                .map(|(name, value)| (name.as_bytes().into(), Some(value.as_bytes().into())))
                .collect(),
        }
    }

    #[cfg(unix)]
    pub fn set_exec(&mut self, mut exec: Exec) {
        use std::os::unix::ffi::OsStringExt;

        self.program = OsString::from_vec(exec.program.into());
        self.arg0 = Some(OsString::from_vec(exec.args.remove(0).into()));
        self.args = exec.args.into_iter().map(|arg| OsString::from_vec(arg.into())).collect();
        self.envs = exec
            .envs
            .into_iter()
            .map(|(name, value)| {
                (
                    OsString::from_vec(name.into()),
                    OsString::from_vec(value.unwrap_or_default().into()),
                )
            })
            .collect()
    }

    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Command {
        self.envs.remove(key.as_ref());
        self
    }

    pub fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.stderr = Some(cfg.into());
        self
    }

    pub fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.stdout = Some(cfg.into());
        self
    }

    pub fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.stdin = Some(cfg.into());
        self
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Command
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.envs.insert(key.as_ref().to_os_string(), val.as_ref().to_os_string());
        self
    }

    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Command
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.envs.extend(
            vars.into_iter()
                .map(|(key, val)| (key.as_ref().to_os_string(), val.as_ref().to_os_string())),
        );
        self
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command {
        self.cwd = Some(dir.as_ref().to_owned());
        self
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args.extend(args.into_iter().map(|arg| arg.as_ref().to_os_string()));
        self
    }

    #[cfg(unix)]
    pub fn arg0<S>(&mut self, arg: S) -> &mut Command
    where
        S: AsRef<OsStr>,
    {
        self.arg0 = Some(arg.as_ref().to_os_string());
        self
    }

    pub async fn spawn(mut self) -> io::Result<TrackedChild> {
        self.resolve_program()?;
        spawn_impl(self).await
    }

    /// Resolve program name to full path using `PATH` and cwd.
    pub fn resolve_program(&mut self) -> io::Result<()> {
        eprintln!("resolving program: {:?}", self.program);
        let mut path_env: Option<&OsStr> = None;
        for (env_name, env_value) in &self.envs {
            let Some(env_name) = env_name.to_str() else {
                continue;
            };
            if env_name.eq_ignore_ascii_case("path") {
                path_env = Some(env_value.as_ref());
                break;
            }
        }

        self.program = which::which_in(
            self.program.as_os_str(),
            path_env,
            if let Some(cwd) = &self.cwd { cwd.clone() } else { std::env::current_dir()? },
        )
        .map_err(|err| io::Error::new(io::ErrorKind::NotFound, err))?
        .into_os_string();
        eprintln!("resolving program after: {:?}", self.program);
        Ok(())
    }

    pub(crate) fn into_tokio_command(self) -> TokioCommand {
        let mut tokio_cmd = TokioCommand::new(self.program);
        if let Some(cwd) = &self.cwd {
            tokio_cmd.current_dir(cwd);
        }

        #[cfg(unix)]
        if let Some(arg0) = self.arg0 {
            tokio_cmd.arg0(arg0);
        }
        tokio_cmd.args(self.args);
        tokio_cmd.env_clear();
        tokio_cmd.envs(self.envs);

        if let Some(stdin) = self.stdin {
            tokio_cmd.stdin(stdin);
        }

        if let Some(stdout) = self.stdout {
            tokio_cmd.stdout(stdout);
        }

        if let Some(stderr) = self.stderr {
            tokio_cmd.stderr(stderr);
        }

        tokio_cmd
    }
}
