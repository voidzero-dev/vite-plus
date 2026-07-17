#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "proc-macro parsing and generated diagnostics use syn-owned strings"
)]

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use semver::Version;
use syn::{
    Attribute, Error, Expr, ExprLit, Field, Fields, Ident, Item, ItemEnum, ItemStruct, Lit,
    LitChar, LitStr, Meta, Result, Token, Type,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

#[proc_macro_attribute]
pub fn pm_args(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return Error::new(Span::call_site(), "#[pm_args] does not take arguments")
            .into_compile_error()
            .into();
    }

    match pm_args_impl(TokenStream2::from(item)) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn pm_args_impl(item: TokenStream2) -> Result<TokenStream2> {
    match syn::parse2::<Item>(item)? {
        Item::Struct(input) => pm_args_struct_impl(input),
        Item::Enum(input) => pm_args_enum_impl(input),
        input => Err(Error::new(input.span(), "#[pm_args] only supports structs and enums")),
    }
}

fn pm_args_struct_impl(mut input: ItemStruct) -> Result<TokenStream2> {
    let struct_ident = input.ident.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let dialect_ident = Ident::new("__VitePmCliDialect", Span::mixed_site());
    let mut field_support = Vec::new();

    let Fields::Named(fields) = &mut input.fields else {
        return Err(Error::new(
            input.fields.span(),
            "#[pm_args] only supports structs with named fields",
        ));
    };

    for field in &mut fields.named {
        if let Some(support) = process_field(field)? {
            field_support.push(support);
        }
    }

    let diagnose_statements = field_support.iter().map(FieldSupport::to_tokens);

    Ok(quote! {
        #input

        impl #impl_generics crate::resolution::Diagnosis for #struct_ident #ty_generics #where_clause {
            fn diagnose<#dialect_ident: crate::resolution::PackageManagerDialect>(
                mut self,
                dialect: &#dialect_ident,
                diag: &mut crate::resolution::Diagnostics,
            ) -> Self
            {
                #(#diagnose_statements)*
                self
            }
        }
    })
}

fn pm_args_enum_impl(mut input: ItemEnum) -> Result<TokenStream2> {
    let enum_ident = input.ident.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let dialect_ident = Ident::new("__VitePmCliDialect", Span::mixed_site());
    let mut variant_diagnosis = Vec::with_capacity(input.variants.len());

    for variant in &mut input.variants {
        let variant_ident = &variant.ident;
        let variant_cfg_attrs = conditional_attrs(&variant.attrs)?;
        match &mut variant.fields {
            Fields::Unit => {
                variant_diagnosis.push(quote! {
                    #(#variant_cfg_attrs)*
                    Self::#variant_ident => {}
                });
            }
            Fields::Named(fields) => {
                let mut field_support = Vec::new();
                for field in &mut fields.named {
                    if let Some(support) = process_field(field)? {
                        field_support.push(support);
                    }
                }

                let pattern = if field_support.is_empty() {
                    quote!(Self::#variant_ident { .. })
                } else {
                    let bindings = field_support
                        .iter()
                        .enumerate()
                        .map(|(index, support)| {
                            let field = &support.ident;
                            let binding = Ident::new(
                                &format!("__vite_pm_cli_arg_{index}"),
                                Span::mixed_site(),
                            );
                            (field, binding)
                        })
                        .collect::<Vec<_>>();
                    let fields = bindings.iter().zip(field_support.iter()).map(
                        |((field, binding), support)| {
                            let cfg_attrs = &support.cfg_attrs;
                            quote!(#(#cfg_attrs)* #field: #binding)
                        },
                    );
                    quote!(Self::#variant_ident { #(#fields),*, .. })
                };
                let diagnose_statements =
                    field_support.iter().enumerate().map(|(index, support)| {
                        let binding =
                            Ident::new(&format!("__vite_pm_cli_arg_{index}"), Span::mixed_site());
                        support.to_binding_tokens(&binding)
                    });
                variant_diagnosis.push(quote! {
                    #(#variant_cfg_attrs)*
                    #pattern => {
                        #(#diagnose_statements)*
                    }
                });
            }
            Fields::Unnamed(fields) => {
                return Err(Error::new(
                    fields.span(),
                    "#[pm_args] does not support tuple enum variants; use inline named fields",
                ));
            }
        }
    }

    Ok(quote! {
        #input

        impl #impl_generics crate::resolution::Diagnosis for #enum_ident #ty_generics #where_clause {
            fn diagnose<#dialect_ident: crate::resolution::PackageManagerDialect>(
                mut self,
                dialect: &#dialect_ident,
                diag: &mut crate::resolution::Diagnostics,
            ) -> Self
            {
                match &mut self {
                    #(#variant_diagnosis),*
                }
                self
            }
        }
    })
}

fn process_field(field: &mut Field) -> Result<Option<FieldSupport>> {
    let Some(field_ident) = field.ident.clone() else {
        return Err(Error::new(field.span(), "#[pm_args] only supports named fields"));
    };

    let cfg_attrs = conditional_attrs(&field.attrs)?;
    let mut support = None;
    let mut new_attrs = Vec::with_capacity(field.attrs.len());

    for attr in field.attrs.clone() {
        if !attr.path().is_ident("arg") {
            new_attrs.push(attr);
            continue;
        }

        let processed = process_arg_attr(&attr, &field_ident)?;
        new_attrs.push(processed.attr);
        if let Some(attr_support) = processed.support {
            if support.is_some() {
                return Err(Error::new(
                    attr.span(),
                    "fields with support metadata must have exactly one relevant #[arg(...)] attribute",
                ));
            }
            ensure_supported_field_shape(&field.ty)?;
            support = Some(FieldSupport {
                ident: field_ident.clone(),
                display_name: attr_support.display_name,
                clauses: attr_support.clauses,
                cfg_attrs: cfg_attrs.clone(),
            });
        }
    }

    field.attrs = new_attrs;
    Ok(support)
}

fn conditional_attrs(attrs: &[Attribute]) -> Result<Vec<Attribute>> {
    let mut conditional = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("cfg") {
            conditional.push(attr.clone());
            continue;
        }
        if !attr.path().is_ident("cfg_attr") {
            continue;
        }

        let metas = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        let mut metas = metas.into_iter();
        let Some(condition) = metas.next() else {
            continue;
        };
        let nested = metas
            .filter(|meta| meta.path().is_ident("cfg") || meta.path().is_ident("cfg_attr"))
            .collect::<Vec<_>>();
        if !nested.is_empty() {
            conditional.push(syn::parse_quote!(#[cfg_attr(#condition, #(#nested),*)]));
        }
    }
    Ok(conditional)
}

struct ProcessedArgAttr {
    attr: Attribute,
    support: Option<AttrSupport>,
}

struct AttrSupport {
    display_name: String,
    clauses: Vec<Clause>,
}

fn process_arg_attr(attr: &Attribute, field_ident: &Ident) -> Result<ProcessedArgAttr> {
    let metas = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    let mut kept = Vec::new();
    let mut clauses = None;
    let mut long_display = None;
    let mut short_display = None;
    let mut value_display = None;

    for meta in metas {
        if is_not_supported_meta(&meta) {
            let Meta::List(list) = &meta else {
                return Err(Error::new(meta.span(), "not_supported must be a list"));
            };
            if clauses.is_some() {
                return Err(Error::new(
                    meta.span(),
                    "duplicate not_supported(...) in one #[arg(...)]",
                ));
            }
            let parsed = syn::parse2::<ClauseList>(list.tokens.clone())?;
            clauses = Some(parsed.clauses.into_iter().collect());
            continue;
        }

        collect_display_name(
            &meta,
            field_ident,
            &mut long_display,
            &mut short_display,
            &mut value_display,
        );
        kept.push(meta);
    }

    let new_attr: Attribute = syn::parse_quote!(#[arg(#(#kept),*)]);
    let support = if let Some(clauses) = clauses {
        let display_name = long_display.or(short_display).or(value_display).ok_or_else(|| {
            Error::new(
                attr.span(),
                "not_supported(...) requires long, short, or value_name in the same #[arg(...)]",
            )
        })?;
        Some(AttrSupport { display_name, clauses })
    } else {
        None
    };

    Ok(ProcessedArgAttr { attr: new_attr, support })
}

fn is_not_supported_meta(meta: &Meta) -> bool {
    matches!(meta, Meta::List(list) if list.path.is_ident("not_supported"))
}

fn collect_display_name(
    meta: &Meta,
    field_ident: &Ident,
    long_display: &mut Option<String>,
    short_display: &mut Option<String>,
    value_display: &mut Option<String>,
) {
    match meta {
        Meta::Path(path) if path.is_ident("long") => {
            *long_display = Some(format!("--{}", kebab_case(&field_ident.to_string())));
        }
        Meta::Path(path) if path.is_ident("short") => {
            let short = field_ident.to_string().chars().next().unwrap_or_default();
            *short_display = Some(format!("-{short}"));
        }
        Meta::NameValue(name_value) if name_value.path.is_ident("long") => {
            if let Some(value) = lit_str(&name_value.value) {
                *long_display = Some(format!("--{}", value.value()));
            }
        }
        Meta::NameValue(name_value) if name_value.path.is_ident("short") => {
            if let Some(value) = lit_char(&name_value.value) {
                *short_display = Some(format!("-{}", value.value()));
            } else if let Some(value) = lit_str(&name_value.value) {
                *short_display = Some(format!("-{}", value.value()));
            }
        }
        Meta::NameValue(name_value) if name_value.path.is_ident("value_name") => {
            if let Some(value) = lit_str(&name_value.value) {
                *value_display = Some(value.value());
            }
        }
        _ => {}
    }
}

fn lit_str(expr: &Expr) -> Option<LitStr> {
    let Expr::Lit(ExprLit { lit: Lit::Str(value), .. }) = expr else {
        return None;
    };
    Some(value.clone())
}

fn lit_char(expr: &Expr) -> Option<LitChar> {
    let Expr::Lit(ExprLit { lit: Lit::Char(value), .. }) = expr else {
        return None;
    };
    Some(value.clone())
}

fn kebab_case(value: &str) -> String {
    value.replace('_', "-")
}

fn ensure_supported_field_shape(ty: &Type) -> Result<()> {
    if is_bool(ty) || is_vec(ty) || is_option(ty) || is_option_vec(ty) {
        return Ok(());
    }

    Err(Error::new(
        ty.span(),
        "not_supported(...) only supports bool, Option<T>, Vec<T>, and Option<Vec<T>> fields",
    ))
}

fn is_bool(ty: &Type) -> bool {
    path_last_ident(ty).is_some_and(|ident| ident == "bool")
}

fn is_option(ty: &Type) -> bool {
    path_last_ident(ty).is_some_and(|ident| ident == "Option")
}

fn is_vec(ty: &Type) -> bool {
    path_last_ident(ty).is_some_and(|ident| ident == "Vec")
}

fn is_option_vec(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    let Some(segment) = path.path.segments.last() else {
        return false;
    };
    if segment.ident != "Option" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    args.args.iter().any(|arg| {
        let syn::GenericArgument::Type(ty) = arg else {
            return false;
        };
        is_vec(ty)
    })
}

fn path_last_ident(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.path.segments.last().map(|segment| segment.ident.to_string())
}

#[derive(Clone)]
struct FieldSupport {
    ident: Ident,
    display_name: String,
    clauses: Vec<Clause>,
    cfg_attrs: Vec<Attribute>,
}

impl FieldSupport {
    fn to_tokens(&self) -> TokenStream2 {
        let ident = &self.ident;
        self.to_target_tokens(quote!(self.#ident))
    }

    fn to_binding_tokens(&self, binding: &Ident) -> TokenStream2 {
        self.to_target_tokens(quote!(*#binding))
    }

    fn to_target_tokens(&self, target: TokenStream2) -> TokenStream2 {
        let cfg_attrs = &self.cfg_attrs;
        let display_name = &self.display_name;
        let clauses = self.clauses.iter().map(Clause::to_tokens);
        let violation = quote! {
            crate::resolution::PmSupportRule::first_matching(&rules, dialect).map(|rule| {
                diag.unsupported_option(#display_name, rule);
            })
        };

        quote! {
            #(#cfg_attrs)*
            {
                let rules = [#(#clauses),*];
                if crate::resolution::ArgActivation::is_active(&#target)
                    && (#violation).is_some()
                {
                    #target = Default::default();
                }
            }
        }
    }
}

#[derive(Clone)]
struct Clause {
    manager: Manager,
    op: Option<VersionOperator>,
    original_version: Option<String>,
    normalized_version: Option<String>,
}

impl Clause {
    fn to_tokens(&self) -> TokenStream2 {
        let manager = self.manager.to_tokens();
        match (&self.op, &self.original_version, &self.normalized_version) {
            (Some(op), Some(original), Some(normalized)) => {
                let op = op.to_tokens();
                quote! {
                    crate::resolution::PmSupportRule::version(
                        #manager,
                        #op,
                        #original,
                        ::semver::Version::parse(#normalized).expect("pm_args emitted a valid semver version"),
                    )
                }
            }
            _ => quote! {
                crate::resolution::PmSupportRule::manager(#manager)
            },
        }
    }
}

#[derive(Clone)]
enum Manager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl Manager {
    fn to_tokens(&self) -> TokenStream2 {
        match self {
            Self::Npm => quote!("npm"),
            Self::Pnpm => quote!("pnpm"),
            Self::Yarn => quote!("yarn"),
            Self::Bun => quote!("bun"),
        }
    }
}

#[derive(Clone)]
enum VersionOperator {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
}

impl VersionOperator {
    fn to_tokens(&self) -> TokenStream2 {
        match self {
            Self::Less => quote!(crate::resolution::VersionOperator::Less),
            Self::LessEqual => quote!(crate::resolution::VersionOperator::LessEqual),
            Self::Greater => quote!(crate::resolution::VersionOperator::Greater),
            Self::GreaterEqual => quote!(crate::resolution::VersionOperator::GreaterEqual),
            Self::Equal => quote!(crate::resolution::VersionOperator::Equal),
        }
    }
}

struct ClauseList {
    clauses: Punctuated<Clause, Token![,]>,
}

impl Parse for ClauseList {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self { clauses: Punctuated::parse_terminated(input)? })
    }
}

impl Parse for Clause {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let manager_ident = input.parse::<Ident>()?;
        let manager = match manager_ident.to_string().as_str() {
            "npm" => Manager::Npm,
            "pnpm" => Manager::Pnpm,
            "yarn" => Manager::Yarn,
            "bun" => Manager::Bun,
            _ => {
                return Err(Error::new(
                    manager_ident.span(),
                    "unknown package manager in support metadata; expected npm, pnpm, yarn, or bun",
                ));
            }
        };

        if input.is_empty() || input.peek(Token![,]) {
            return Ok(Self {
                manager,
                op: None,
                original_version: None,
                normalized_version: None,
            });
        }

        let op = if input.peek(Token![<=]) {
            input.parse::<Token![<=]>()?;
            VersionOperator::LessEqual
        } else if input.peek(Token![>=]) {
            input.parse::<Token![>=]>()?;
            VersionOperator::GreaterEqual
        } else if input.peek(Token![<]) {
            input.parse::<Token![<]>()?;
            VersionOperator::Less
        } else if input.peek(Token![>]) {
            input.parse::<Token![>]>()?;
            VersionOperator::Greater
        } else if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            VersionOperator::Equal
        } else {
            return Err(Error::new(input.span(), "expected version operator <, <=, >, >=, or ="));
        };

        let version = input.parse::<LitStr>()?;
        let original_version = version.value();
        let normalized_version = normalize_version_literal(&version)?;

        Ok(Self {
            manager,
            op: Some(op),
            original_version: Some(original_version),
            normalized_version: Some(normalized_version),
        })
    }
}

fn normalize_version_literal(version: &LitStr) -> Result<String> {
    let value = version.value();
    let normalized = if value.contains('-') || value.contains('+') {
        Version::parse(&value).map_err(|error| Error::new(version.span(), error.to_string()))?;
        value
    } else {
        let parts = value.split('.').collect::<Vec<_>>();
        if parts.is_empty() || parts.len() > 3 || parts.iter().any(|part| part.is_empty()) {
            return Err(Error::new(version.span(), "version literal must be a semver prefix"));
        }
        let mut numbers = Vec::with_capacity(3);
        for part in parts {
            let number = part
                .parse::<u64>()
                .map_err(|_| Error::new(version.span(), "version literal must be numeric"))?;
            numbers.push(number);
        }
        while numbers.len() < 3 {
            numbers.push(0);
        }
        format!("{}.{}.{}", numbers[0], numbers[1], numbers[2])
    };

    Version::parse(&normalized).map_err(|error| Error::new(version.span(), error.to_string()))?;
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::quote;

    use super::*;

    #[test]
    fn strips_not_support_from_arg_attr() {
        let output = pm_args_impl(quote! {
            #[derive(Clone)]
            struct Demo {
                #[arg(long, value_name = "VALUE", not_supported(npm, yarn < "2"))]
                foo: bool,
            }
        })
        .unwrap()
        .to_string();

        assert!(!output.contains("not_supported"));
        assert!(output.contains("value_name"));
        assert!(output.contains("PmSupportRule"));
    }

    #[test]
    fn expands_unit_and_inline_named_enum_variants() {
        let output = pm_args_impl(quote! {
            #[derive(clap::Subcommand, Clone)]
            enum Demo {
                Ping,
                List {
                    #[arg(long, not_supported(bun))]
                    json: bool,
                },
            }
        })
        .unwrap()
        .to_string();

        assert!(!output.contains("not_supported"));
        assert!(output.contains("impl crate :: resolution :: Diagnosis for Demo"));
        assert!(output.contains("match & mut self"));
        assert!(output.contains("Self :: Ping"));
        assert!(output.contains("Self :: List"));
    }

    #[test]
    fn expands_inline_named_enum_variant_without_support_metadata() {
        let output = pm_args_impl(quote! {
            #[derive(clap::Subcommand, Clone)]
            enum Demo {
                List {
                    #[arg(long)]
                    json: bool,
                },
            }
        })
        .unwrap()
        .to_string();

        assert!(output.contains("Self :: List { .. }"));
    }

    #[test]
    fn enum_bindings_do_not_shadow_generated_names() {
        let output = pm_args_impl(quote! {
            #[derive(clap::Subcommand, Clone)]
            enum Demo {
                List {
                    #[arg(long, not_supported(bun))]
                    diag: bool,
                    #[arg(long, not_supported(bun))]
                    dialect: bool,
                    #[arg(long, not_supported(bun))]
                    rules: bool,
                },
            }
        })
        .unwrap()
        .to_string();

        assert!(output.contains("diag : __vite_pm_cli_arg_0"));
        assert!(output.contains("dialect : __vite_pm_cli_arg_1"));
        assert!(output.contains("rules : __vite_pm_cli_arg_2"));
    }

    #[test]
    fn rejects_tuple_enum_variants() {
        let error = pm_args_impl(quote! {
            #[derive(clap::Subcommand, Clone)]
            enum Demo {
                List(ListArgs),
            }
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("does not support tuple enum variants"));
        assert!(error.contains("use inline named fields"));
    }

    #[test]
    fn rejects_unknown_manager_token() {
        let error = pm_args_impl(quote! {
            #[derive(Clone)]
            struct Demo {
                #[arg(long, not_supported(corepack))]
                foo: bool,
            }
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("unknown package manager"));
    }

    #[test]
    fn rejects_gated_field_without_display_name() {
        let error = pm_args_impl(quote! {
            #[derive(Clone)]
            struct Demo {
                #[arg(not_supported(npm))]
                foo: bool,
            }
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("requires long, short, or value_name"));
    }

    #[test]
    fn rejects_unsupported_gated_field_shape() {
        let error = pm_args_impl(quote! {
            #[derive(Clone)]
            struct Demo {
                #[arg(long, not_supported(npm))]
                foo: String,
            }
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("only supports bool"));
    }

    #[test]
    fn normalizes_version_prefixes() {
        let version = LitStr::new("2.1", Span::call_site());
        assert_eq!(normalize_version_literal(&version).unwrap(), "2.1.0");
    }

    #[test]
    fn keeps_full_prerelease_versions() {
        let version = LitStr::new("2.0.0-rc.1", Span::call_site());
        assert_eq!(normalize_version_literal(&version).unwrap(), "2.0.0-rc.1");
    }
}
