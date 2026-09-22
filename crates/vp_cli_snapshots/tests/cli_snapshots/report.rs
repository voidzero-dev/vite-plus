//! Optional per-process artifacts. Each trial owns its files, including under nextest.
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const OUTPUT_LIMIT: usize = 1024 * 1024;

pub struct Report {
    directory: Option<PathBuf>,
    name: String,
    started: Instant,
    phases: RefCell<Vec<serde_json::Value>>,
    output: RefCell<String>,
    output_truncated: Cell<bool>,
    actual: RefCell<Option<String>>,
}

impl Report {
    pub fn new(directory: Option<PathBuf>, name: String) -> Self {
        Self {
            directory,
            name,
            started: Instant::now(),
            phases: RefCell::new(Vec::new()),
            output: RefCell::new(String::new()),
            output_truncated: Cell::new(false),
            actual: RefCell::new(None),
        }
    }

    pub fn phase(&self, name: impl Into<String>) -> Phase<'_> {
        Phase { report: self, name: name.into(), started: Instant::now() }
    }

    fn record(&self, name: &str, elapsed: Duration) {
        if self.directory.is_some() {
            self.phases.borrow_mut().push(serde_json::json!({
                "name": name,
                "duration_ms": elapsed.as_secs_f64() * 1000.0,
            }));
        }
    }

    /// Keep bounded, unredacted rendered output, including successful hidden steps.
    /// It is only written when the trial fails; this is not the raw PTY byte stream.
    pub fn capture(&self, command: &str, output: &str) {
        if self.directory.is_none() {
            return;
        }
        let mut captured = self.output.borrow_mut();
        captured.push_str(&format!("\n$ {command}\n"));
        captured.push_str(output);
        if captured.len() > OUTPUT_LIMIT {
            self.output_truncated.set(true);
            let mut start = captured.len() - OUTPUT_LIMIT;
            while !captured.is_char_boundary(start) {
                start += 1;
            }
            captured.drain(..start);
        }
    }

    pub fn actual(&self, contents: &str) {
        if self.directory.is_some() {
            *self.actual.borrow_mut() = Some(contents.to_owned());
        }
    }

    pub fn finish(&self, error: Option<&str>, expected: Option<&Path>) -> Result<(), String> {
        let Some(directory) = &self.directory else { return Ok(()) };
        let write = || -> std::io::Result<()> {
            std::fs::create_dir_all(directory)?;
            let timings = serde_json::json!({
                "schema_version": 1,
                "name": self.name,
                "output_truncated": self.output_truncated.get(),
                "status": if error.is_some() { "failed" } else { "passed" },
                "duration_ms": self.started.elapsed().as_secs_f64() * 1000.0,
                "phases": *self.phases.borrow(),
            });
            std::fs::write(directory.join("timing.json"), serde_json::to_vec_pretty(&timings)?)?;
            let Some(error) = error else { return Ok(()) };
            std::fs::write(directory.join("error.txt"), error)?;
            std::fs::write(directory.join("output.txt"), self.output.borrow().as_bytes())?;
            if let Some(actual) = self.actual.borrow().as_ref() {
                std::fs::write(directory.join("actual.md"), actual)?;
            }
            if let Some(expected) = expected {
                match std::fs::read(expected) {
                    Ok(contents) => std::fs::write(directory.join("expected.md"), contents)?,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        };
        write().map_err(|error| {
            format!("failed to write artifacts to {}: {error}", directory.display())
        })
    }
}

impl Drop for Report {
    fn drop(&mut self) {
        if std::thread::panicking() {
            // Do not mask the original panic if saving diagnostics also fails.
            let _ = self.finish(Some("snapshot runner panicked; see the test log"), None);
        }
    }
}

pub struct Phase<'a> {
    report: &'a Report,
    name: String,
    started: Instant,
}

impl Drop for Phase<'_> {
    fn drop(&mut self) {
        self.report.record(&self.name, self.started.elapsed());
    }
}
