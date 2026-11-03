//! The `oxcNode` namespace: thin forwarders to the vendored oxc-node, which the binding
//! links with `default-features = false`, so it registers no NAPI exports of its own.
//! Only what the `vpx <script>` loader in `packages/cli/src/script-*.ts` uses is exported.
//! `enableWatchReport` and `takeWatchDependencies` feed watch mode the tsconfig files
//! oxc-node read and the paths a failed resolve looked for.

use ::oxc_node::{
    FrozenTsconfig, LoadContext, LoadFnOutput, OxcResolveOptions, ResolveContext, ResolveFnOutput,
};
use napi::{
    Either, Env, Result,
    bindgen_prelude::{FnArgs, Function, Object, PromiseRaw},
};
use napi_derive::napi;

/// What oxc-node's own module hooks do, which `default-features = false` leaves to the
/// host: honor `OXC_LOG`, and keep `using` native on Node.js 24 and later.
#[napi(module_exports)]
pub fn init_oxc_node(exports: Object, env: Env) -> Result<()> {
    ::oxc_node::init_tracing();
    ::oxc_node::detect_runtime_syntax(exports, env)
}

#[napi(namespace = "oxcNode")]
pub fn set_tsconfig_path(path: Option<String>) -> Result<()> {
    ::oxc_node::set_tsconfig_path(path)
}

/// Freeze the effective config before registering the off-thread ESM hooks.
#[napi(namespace = "oxcNode")]
pub fn freeze_tsconfig(env: Env, cwd: String) -> Result<FrozenTsconfig> {
    ::oxc_node::freeze_tsconfig_js(env, cwd)
}

/// Apply a frozen selection, including discovery without the receiving thread's env.
#[napi(namespace = "oxcNode")]
pub fn initialize_loader(path: Option<String>, source: Option<String>) -> Result<()> {
    ::oxc_node::initialize_loader_js(path, source)
}

#[napi(namespace = "oxcNode")]
pub fn set_transform_deps(mode: Option<String>) -> Result<()> {
    ::oxc_node::set_transform_deps(mode)
}

#[napi(namespace = "oxcNode")]
pub fn freeze_transform_deps() -> String {
    ::oxc_node::freeze_transform_deps()
}

#[napi(namespace = "oxcNode")]
pub fn transforms_dep(path: String) -> bool {
    ::oxc_node::transforms_dep(path)
}

#[napi(namespace = "oxcNode")]
pub fn enable_watch_report() {
    ::oxc_node::enable_watch_report();
}

#[napi(namespace = "oxcNode")]
pub fn take_watch_dependencies() -> Vec<String> {
    ::oxc_node::take_watch_dependencies()
}

#[napi(namespace = "oxcNode")]
#[allow(clippy::type_complexity)]
pub fn create_resolve<'env>(
    env: &'env Env,
    specifier: String,
    context: ResolveContext,
    next_resolve: Function<
        'env,
        FnArgs<(String, Option<ResolveContext>)>,
        Either<ResolveFnOutput, PromiseRaw<'env, ResolveFnOutput>>,
    >,
) -> Result<Either<ResolveFnOutput, PromiseRaw<'env, ResolveFnOutput>>> {
    // Only oxc-node's WASI build reads the options, so this addon passes none.
    let options = OxcResolveOptions { get_current_directory: None };
    ::oxc_node::create_resolve(env, options, specifier, context, next_resolve)
}

#[napi(namespace = "oxcNode")]
#[allow(clippy::type_complexity)]
pub fn load<'env>(
    url: String,
    context: LoadContext,
    next_load: Function<
        'env,
        FnArgs<(String, Option<LoadContext>)>,
        Either<LoadFnOutput, PromiseRaw<'env, LoadFnOutput>>,
    >,
    helper_module_name: Option<String>,
) -> Result<Either<LoadFnOutput, PromiseRaw<'env, LoadFnOutput>>> {
    ::oxc_node::load(url, context, next_load, helper_module_name)
}

/// A file transformed for the CommonJS loader.
#[napi(object, namespace = "oxcNode")]
pub struct TransformOutput {
    pub source: String,
    /// The source map as a JSON string.
    pub source_map: Option<String>,
}

#[napi(namespace = "oxcNode")]
pub struct OxcTransformer {
    inner: ::oxc_node::OxcTransformer,
}

#[napi(namespace = "oxcNode")]
impl OxcTransformer {
    #[napi(constructor)]
    pub fn new(cwd: Option<String>, helper_module_name: Option<String>) -> Self {
        Self { inner: ::oxc_node::OxcTransformer::new(cwd, helper_module_name) }
    }

    #[napi]
    pub fn transform(
        &self,
        path: String,
        source: Either<String, &[u8]>,
    ) -> Result<TransformOutput> {
        let output = self.inner.transform(path, source)?;
        Ok(TransformOutput { source: output.source(), source_map: output.source_map() })
    }
}
