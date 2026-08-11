use std::process::ExitStatus;

use vt_path::AbsolutePathBuf;

use crate::{commands::delegate, error::Error, js_executor::JsExecutor};

pub async fn execute(
    cwd: AbsolutePathBuf,
    tools: Vec<String>,
    json: bool,
    global: bool,
    raw_subcommand: Option<&str>,
) -> Result<ExitStatus, Error> {
    if !global && JsExecutor::resolve_local_vite_plus(&cwd).is_some() {
        let mut args = tools;
        if json {
            args.push("--json".to_string());
        }
        return delegate::execute(cwd, "toolchain", &args, raw_subcommand).await;
    }

    let scripts_dir = JsExecutor::new(None).get_scripts_dir()?;
    let package_dir = scripts_dir.parent().ok_or(Error::JsScriptsDirNotFound)?;
    let manifest_path = scripts_dir.join("toolchain.json");
    let manifest = vp_toolchain::load_manifest(&manifest_path)?;
    let version = vp_toolchain::root_version(&manifest)
        .ok_or_else(|| Error::Other("toolchain manifest does not contain vite-plus".into()))?;
    let source = vp_toolchain::Source {
        scope: vp_toolchain::Scope::Global,
        path: package_dir.as_path().to_string_lossy().into_owned().into(),
        vite_plus_version: version.into(),
    };
    let report = match vp_toolchain::build_report(&manifest, &tools, source) {
        Ok(report) => report,
        Err(vp_toolchain::ToolchainError::UnknownFilter(filter)) => {
            let message = format!("`{filter}` is not in the Vite+ toolchain");
            if json {
                vp_shared::output::raw_stderr(&format!("error: {message}"));
            } else {
                vp_shared::output::error(&message);
                vp_shared::output::raw_stderr(&format!(
                    "hint: run `vp why {filter}` to show project dependencies"
                ));
            }
            return Ok(crate::cli::exit_status(1));
        }
        Err(error) => return Err(error.into()),
    };

    let rendered =
        if json { vp_toolchain::render_json(&report)? } else { vp_toolchain::render(&report) };
    vp_shared::output::raw_inline(&rendered);
    Ok(ExitStatus::default())
}
