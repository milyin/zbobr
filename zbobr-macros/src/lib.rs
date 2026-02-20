use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::{
    parse_macro_input, Attribute, Fields, Ident, ItemStruct, Lit, LitStr, Meta, Token, Type,
    TypePath,
};

#[proc_macro]
pub fn config_struct(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    match expand_config_struct(item) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_config_struct(item: ItemStruct) -> syn::Result<TokenStream2> {
    let ItemStruct {
        attrs,
        vis,
        ident,
        fields,
        ..
    } = item;

    let fields_named = match fields {
        Fields::Named(named) => named.named,
        _ => {
            return Err(syn::Error::new_spanned(
                ident,
                "config_struct! only supports structs with named fields",
            ))
        }
    };

    let (_, prefix_kebab) = struct_prefixes(&ident);

    let toml_ident = format_ident!("{}Toml", ident);
    let args_ident = format_ident!("{}Args", ident);

    let mut toml_fields = Vec::new();
    let mut args_fields = Vec::new();
    let mut merge_fields = Vec::new();
    let mut has_override_checks = Vec::new();

    for field in fields_named {
        let field_ident = field.ident.clone().ok_or_else(|| {
            syn::Error::new_spanned(&field, "config_struct! fields must be named")
        })?;
        let field_vis = field.vis.clone();
        let field_ty = field.ty.clone();

        let (other_attrs, arg_metas, config_meta) = partition_field_attrs(&field.attrs)?;
        // Reuse doc/other attributes on both structs.
        let args_attrs = other_attrs.clone();
        let rename_attr = config_meta
            .toml_rename
            .as_ref()
            .map(|name| quote!(#[serde(rename = #name)]));
        let rename_attr_tokens = rename_attr.clone().unwrap_or_else(TokenStream2::new);

        let field_snake = field_ident.to_string().to_snake_case();
        let field_kebab = field_snake.replace('_', "-");

        if config_meta.nested {
            let nested_toml_ty = config_meta
                .nested_toml_ty
                .clone()
                .unwrap_or(type_with_suffix(&field_ty, "Toml")?);
            let nested_args_ty = config_meta
                .nested_args_ty
                .clone()
                .unwrap_or(type_with_suffix(&field_ty, "Args")?);

            let heading_prefix = config_meta
                .heading_prefix
                .clone()
                .or_else(|| {
                    let source_ty = Type::Path(nested_args_ty.clone());
                    type_prefixes(&source_ty).ok().map(|(_, kebab)| kebab)
                })
                .unwrap_or_else(|| prefix_kebab.clone());

            let heading_text = config_meta
                .help_heading
                .clone()
                .or_else(|| doc_comment(&other_attrs));
            let heading = format_help_heading(&heading_prefix, heading_text.as_deref());
            let heading_lit = LitStr::new(&heading, Span::call_site());

            if !config_meta.skip_toml {
                toml_fields.push(quote! {
                    #rename_attr_tokens
                    #(#other_attrs)*
                    #field_vis #field_ident: Option<#nested_toml_ty>,
                });

                merge_fields.push(quote! {
                    #field_ident: {
                        let arg_value = args.#field_ident;
                        match self.#field_ident {
                            Some(current) => Some(current.merge_with_args(arg_value)),
                            None => {
                                if arg_value.has_overrides() {
                                    Some(<#nested_toml_ty>::default().merge_with_args(arg_value))
                                } else {
                                    None
                                }
                            }
                        }
                    },
                });
            }

            args_fields.push(quote! {
                #(#args_attrs)*
                #[command(flatten, next_help_heading = #heading_lit)]
                #field_vis #field_ident: #nested_args_ty,
            });

            has_override_checks.push(quote! {
                self.#field_ident.has_overrides()
            });
        } else {
            let arg_long_value = format!("{}-{}", prefix_kebab, field_kebab);
            let arg_long_lit = LitStr::new(&arg_long_value, Span::call_site());

            let mut arg_entries: Vec<TokenStream2> = Vec::new();
            arg_entries.push(quote!(name = #arg_long_lit));
            arg_entries.push(quote!(long = #arg_long_lit));
            for meta in arg_metas {
                if should_skip_arg_meta(&meta) {
                    continue;
                }
                arg_entries.push(quote!(#meta));
            }

            let arg_attr = quote! {
                #[arg( #(#arg_entries),* )]
            };

            if !config_meta.skip_toml {
                toml_fields.push(quote! {
                    #rename_attr_tokens
                    #(#other_attrs)*
                    #field_vis #field_ident: Option<#field_ty>,
                });

                merge_fields.push(quote! {
                    #field_ident: args.#field_ident.or(self.#field_ident),
                });
            }

            args_fields.push(quote! {
                #(#args_attrs)*
                #arg_attr
                #field_vis #field_ident: Option<#field_ty>,
            });

            has_override_checks.push(quote! {
                self.#field_ident.is_some()
            });
        }
    }

    let tokens = quote! {
        #(#attrs)*
        #[derive(Debug, Clone, ::serde::Deserialize, Default)]
        #[serde(default, deny_unknown_fields)]
        #vis struct #toml_ident {
            #(#toml_fields)*
        }

        #(#attrs)*
        #[derive(Debug, Clone, ::clap::Args, Default)]
        #vis struct #args_ident {
            #(#args_fields)*
        }

        impl #toml_ident {
            pub fn merge_with_args(self, args: #args_ident) -> Self {
                Self {
                    #(#merge_fields)*
                }
            }
        }

        impl #args_ident {
            pub fn has_overrides(&self) -> bool {
                false #( || #has_override_checks )*
            }
        }
    };

    Ok(tokens)
}

#[derive(Default)]
struct FieldConfig {
    nested: bool,
    help_heading: Option<String>,
    skip_toml: bool,
    nested_args_ty: Option<TypePath>,
    nested_toml_ty: Option<TypePath>,
    heading_prefix: Option<String>,
    toml_rename: Option<String>,
}

fn partition_field_attrs(
    attrs: &[Attribute],
) -> syn::Result<(Vec<Attribute>, Vec<Meta>, FieldConfig)> {
    let mut other = Vec::new();
    let mut arg_items = Vec::new();
    let mut config_meta = FieldConfig::default();

    for attr in attrs {
        if attr.path().is_ident("arg") {
            let metas: Punctuated<Meta, Token![,]> =
                attr.parse_args_with(Punctuated::parse_terminated)?;
            arg_items.extend(metas.into_iter());
        } else if attr.path().is_ident("config") {
            parse_config_meta(attr, &mut config_meta)?;
        } else {
            other.push(attr.clone());
        }
    }

    Ok((other, arg_items, config_meta))
}

fn parse_config_meta(attr: &Attribute, config: &mut FieldConfig) -> syn::Result<()> {
    let metas: Punctuated<Meta, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;
    for meta in metas {
        match meta {
            Meta::Path(path) if path.is_ident("nested") => config.nested = true,
            Meta::Path(path) if path.is_ident("skip_toml") => config.skip_toml = true,
            Meta::NameValue(name_value) if name_value.path.is_ident("help_heading") => {
                if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(lit), .. }) = name_value.value
                {
                    config.help_heading = Some(lit.value());
                } else {
                    return Err(syn::Error::new_spanned(
                        name_value,
                        "help_heading must be a string literal",
                    ));
                }
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("heading_prefix") => {
                if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(lit), .. }) = name_value.value
                {
                    config.heading_prefix = Some(lit.value());
                } else {
                    return Err(syn::Error::new_spanned(
                        name_value,
                        "heading_prefix must be a string literal",
                    ));
                }
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("args_type") => {
                let parsed = parse_type_path(&name_value.value, "args_type")?;
                config.nested_args_ty = Some(parsed);
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("toml_type") => {
                let parsed = parse_type_path(&name_value.value, "toml_type")?;
                config.nested_toml_ty = Some(parsed);
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("toml_rename") => {
                if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(lit), .. }) = name_value.value
                {
                    config.toml_rename = Some(lit.value());
                } else {
                    return Err(syn::Error::new_spanned(
                        name_value,
                        "toml_rename must be a string literal",
                    ));
                }
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    meta,
                    "Unsupported #[config(...)] attribute",
                ))
            }
        }
    }
    Ok(())
}

fn should_skip_arg_meta(meta: &Meta) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("long") || path.is_ident("name"),
        Meta::NameValue(name_value) => {
            name_value.path.is_ident("long") || name_value.path.is_ident("name")
        }
        Meta::List(list) => list.path.is_ident("long") || list.path.is_ident("name"),
    }
}

fn struct_prefixes(ident: &Ident) -> (String, String) {
    let mut snake = ident.to_string().to_snake_case();
    if let Some(stripped) = snake.strip_prefix("zbobr_") {
        snake = stripped.to_string();
    }
    snake = snake.replace("_backend", "");
    while snake.contains("__") {
        snake = snake.replace("__", "_");
    }
    if let Some(stripped) = snake.strip_prefix('_') {
        if !stripped.is_empty() {
            snake = stripped.to_string();
        }
    }
    if snake.is_empty() {
        snake = "config".to_string();
    }
    let kebab = snake.replace('_', "-");
    (snake, kebab)
}

fn type_prefixes(ty: &Type) -> syn::Result<(String, String)> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(ident) = path.segments.last().map(|s| &s.ident) {
            return Ok(struct_prefixes(ident));
        }
    }
    Err(syn::Error::new_spanned(ty, "config_struct! only supports named type paths"))
}

fn type_with_suffix(ty: &Type, suffix: &str) -> syn::Result<TypePath> {
    if let Type::Path(type_path) = ty {
        let mut new_path = type_path.clone();
        if let Some(last) = new_path.path.segments.last_mut() {
            last.ident = format_ident!("{}{}", last.ident, suffix);
        }
        return Ok(new_path);
    }
    Err(syn::Error::new_spanned(
        ty,
        "config_struct! fields must use a path type to derive nested names",
    ))
}

fn parse_type_path(expr: &syn::Expr, label: &str) -> syn::Result<TypePath> {
    let ty: Type = syn::parse2(expr.to_token_stream())?;
    if let Type::Path(tp) = ty {
        Ok(tp)
    } else {
        Err(syn::Error::new_spanned(
            expr,
            format!("{label} must be a type path"),
        ))
    }
}

fn doc_comment(attrs: &[Attribute]) -> Option<String> {
    let mut docs = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        let mut found: Option<String> = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("doc") {
                let lit: LitStr = meta.value()?.parse()?;
                found = Some(lit.value());
            }
            Ok(())
        });
        if let Some(value) = found {
            docs.push(value);
        }
    }
    if docs.is_empty() {
        None
    } else {
        let joined = docs.join(" ");
        let trimmed = joined.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}

fn format_help_heading(prefix_kebab: &str, description: Option<&str>) -> String {
    let prefix = prefix_kebab.replace('-', ".");
    match description {
        Some(desc) if !desc.trim().is_empty() => format!("[{prefix}] {desc}"),
        _ => format!("[{prefix}]"),
    }
}
