//! The `oxcNode` namespace: thin forwarders to the vendored oxc-node, which the binding
//! links with `default-features = false`, so it registers no NAPI exports of its own.
//! Only what the `vpx <script>` loader in `packages/cli/src/script-*.ts` uses is exported.

use napi::{
    Either, Env, Result,
    bindgen_prelude::{FnArgs, Function, Object, PromiseRaw},
};
use napi_derive::napi;
use ::oxc_node::{LoadContext, LoadFnOutput, OxcResolveOptions, ResolveContext, ResolveFnOutput};

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

#[napi(namespace = "oxcNode")]
pub fn set_transform_deps(mode: Option<String>) -> Result<()> {
    ::oxc_node::set_transform_deps(mode)
}

#[napi(namespace = "oxcNode")]
pub fn transforms_dep(path: String) -> bool {
    ::oxc_node::transforms_dep(path)
}

#[napi(namespace = "oxcNode")]
#[allow(clippy::type_complexity)]
pub fn create_resolve<'env>(
    env: &'env Env,
    options: OxcResolveOptions,
    specifier: String,
    context: ResolveContext,
    next_resolve: Function<
        'env,
        FnArgs<(String, Option<ResolveContext>)>,
        Either<ResolveFnOutput, PromiseRaw<'env, ResolveFnOutput>>,
    >,
) -> Result<Either<ResolveFnOutput, PromiseRaw<'env, ResolveFnOutput>>> {
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
    pub fn transform(&self, path: String, source: Either<String, &[u8]>) -> Result<TransformOutput> {
        let output = self.inner.transform(path, source)?;
        Ok(TransformOutput { source: output.source(), source_map: output.source_map() })
    }
}
