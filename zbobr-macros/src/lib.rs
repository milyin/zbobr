use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Fields, GenericArgument, ItemStruct, Lit, LitStr, Meta, Token, Type, TypePath,
    parse_macro_input, punctuated::Punctuated,
};

#[proc_macro_attribute]
pub fn config_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(Span::call_site(), "config_struct does not take arguments")
            .to_compile_error()
            .into();
    }

    let parsed = parse_macro_input!(item as ItemStruct);
    match expand_config_struct(parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_config_struct(item: ItemStruct) -> syn::Result<TokenStream2> {
    let ItemStruct {
        attrs,
        vis,
        mut ident,
        fields,
        generics,
        ..
    } = item;

    // for consistency we want configuration structs to end in "Config";
    // if the user didn't include that suffix we append it and create a
    // type alias from the original name back to the new one for
    // compatibility.
    let orig_ident = ident.clone();
    if !orig_ident.to_string().ends_with("Config") {
        ident = format_ident!("{}Config", orig_ident);
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields_named = match fields {
        Fields::Named(named) => named.named,
        _ => {
            return Err(syn::Error::new_spanned(
                ident,
                "config_struct only supports structs with named fields",
            ));
        }
    };

    let toml_ident = format_ident!("{}Toml", ident);
    let args_ident = format_ident!("{}Args", ident);
    let derived_args_ident = format_ident!("{}ArgsDerived", ident);

    // if we renamed the struct we expose an alias from the original name
    // back to the config name so code using the old identifier continues to
    // compile.
    let alias_orig_decl = if orig_ident != ident {
        quote! { pub type #orig_ident #ty_generics = #ident #ty_generics; }
    } else {
        TokenStream2::new()
    };

    let ident_str = ident.to_string();
    let alias_base = ident_str
        .strip_suffix("Config")
        .map(|s| format_ident!("{}", s));
    let alias_toml_ident = alias_base
        .as_ref()
        .map(|base| format_ident!("{}Toml", base));
    let alias_args_ident = alias_base
        .as_ref()
        .map(|base| format_ident!("{}Args", base));

    let mut toml_fields = Vec::new();
    let mut derived_args_fields = Vec::new();
    let mut plain_args_fields = Vec::new();
    let mut merge_fields = Vec::new();
    let mut has_override_checks = Vec::new();
    let mut base_fields = Vec::new();
    let mut namespace_steps = Vec::new();
    let mut args_copy_fields = Vec::new();
    let mut args_update_fields = Vec::new();
    let mut into_config_setup = Vec::new();
    let mut into_config_required = Vec::new();
    let mut into_config_fields = Vec::new();

    for field in fields_named {
        let field_ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new_spanned(&field, "config_struct fields must be named"))?;
        let field_vis = field.vis.clone();
        let field_ty = field.ty.clone();

        let (other_attrs, arg_metas, config_meta) = partition_field_attrs(&field.attrs)?;
        let doc_help = doc_comment(&other_attrs);
        base_fields.push(quote! {
            #(#other_attrs)*
            #field_vis #field_ident: #field_ty,
        });
        // Reuse doc/other attributes on both structs.
        let args_attrs = other_attrs.clone();
        let rename_attr = config_meta
            .toml_rename
            .as_ref()
            .map(|name| quote!(#[serde(rename = #name)]));
        let rename_attr_tokens = rename_attr.clone().unwrap_or_else(TokenStream2::new);

        let field_snake = field_ident.to_string().to_snake_case();
        let field_kebab = field_snake.replace('_', "-");
        let field_kebab_lit = LitStr::new(&field_kebab, Span::call_site());

        let is_nested = config_meta.nested;

        if is_nested {
            let base_is_option = option_inner_type(&field_ty);
            let suffix_target_ty = base_is_option.as_ref().unwrap_or(&field_ty);

            let nested_toml_ty = config_meta
                .nested_toml_ty
                .clone()
                .unwrap_or(type_with_suffix(suffix_target_ty, "Toml")?);
            let nested_args_ty = config_meta
                .nested_args_ty
                .clone()
                .unwrap_or(type_with_suffix(suffix_target_ty, "Args")?);

            let heading_prefix = config_meta
                .heading_prefix
                .clone()
                .unwrap_or_else(|| field_kebab.clone());

            let heading_text = config_meta
                .help_heading
                .clone()
                .or_else(|| doc_help.clone());
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

            derived_args_fields.push(quote! {
                #(#args_attrs)*
                #[command(flatten, next_help_heading = #heading_lit)]
                #field_vis #field_ident: #nested_args_ty,
            });

            plain_args_fields.push(quote! {
                #(#args_attrs)*
                #field_vis #field_ident: #nested_args_ty,
            });

            args_copy_fields.push(quote! { #field_ident: parsed.#field_ident });
            args_update_fields.push(quote! { self.#field_ident = parsed.#field_ident; });

            has_override_checks.push(quote! {
                self.#field_ident.has_overrides()
            });

            namespace_steps.push(quote! {
                let nested_prefix = if prefix.is_empty() {
                    format!("{}.", #field_kebab_lit)
                } else {
                    format!("{}{}.", prefix, #field_kebab_lit)
                };
                cmd = <#nested_args_ty>::namespace_command(cmd, &nested_prefix);
            });

            if !config_meta.skip_toml {
                into_config_setup.push(quote! {
                    let #field_ident = self.#field_ident
                        .map(|value| value.try_into_config())
                        .transpose()?;
                });

                let init_expr = if base_is_option.is_some() {
                    quote!(#field_ident)
                } else {
                    into_config_required.push(quote! {
                        if #field_ident.is_none() {
                            missing_fields.push(stringify!(#field_ident));
                        }
                    });
                    quote!(#field_ident.unwrap())
                };

                into_config_fields.push(quote! {
                    #field_ident: #init_expr
                });
            }
        } else {
            let arg_name_value = config_meta
                .heading_prefix
                .clone()
                .unwrap_or_else(|| field_kebab.clone());
            let arg_name_lit = LitStr::new(&arg_name_value, Span::call_site());
            let arg_target_id = meta_string_value(&arg_metas, "id")
                .or_else(|| meta_string_value(&arg_metas, "name"))
                .unwrap_or_else(|| arg_name_value.clone());
            let arg_target_id_lit = LitStr::new(&arg_target_id, Span::call_site());
            let has_custom_long = arg_metas.iter().any(|m| is_arg_meta(m, "long"));
            let has_custom_name = arg_metas.iter().any(|m| is_arg_meta(m, "name"));
            let has_custom_help = arg_metas.iter().any(|m| is_arg_meta(m, "help"));

            let mut arg_entries: Vec<TokenStream2> = Vec::new();
            if !has_custom_name {
                arg_entries.push(quote!(name = #arg_name_lit));
            }
            if !has_custom_long {
                arg_entries.push(quote!(long = #arg_name_lit));
            }
            if let Some(help) = doc_help.clone() {
                if !has_custom_help {
                    let help_lit = LitStr::new(&help, Span::call_site());
                    arg_entries.push(quote!(help = #help_lit));
                }
            }
            for meta in arg_metas {
                arg_entries.push(quote!(#meta));
            }

            let arg_attr = quote! {
                #[arg( #(#arg_entries),* )]
            };

            let base_is_option = option_inner_type(&field_ty);
            let value_ty = base_is_option.clone().unwrap_or(field_ty.clone());

            if !config_meta.skip_toml {
                toml_fields.push(quote! {
                    #rename_attr_tokens
                    #(#other_attrs)*
                    #field_vis #field_ident: Option<#value_ty>,
                });

                merge_fields.push(quote! {
                    #field_ident: args.#field_ident.or(self.#field_ident),
                });
            }

            derived_args_fields.push(quote! {
                #(#args_attrs)*
                #arg_attr
                #field_vis #field_ident: Option<#value_ty>,
            });

            plain_args_fields.push(quote! {
                #(#args_attrs)*
                #field_vis #field_ident: Option<#value_ty>,
            });

            args_copy_fields.push(quote! { #field_ident: parsed.#field_ident });
            args_update_fields.push(quote! { self.#field_ident = parsed.#field_ident; });

            has_override_checks.push(quote! {
                self.#field_ident.is_some()
            });

            namespace_steps.push(quote! {
                let lookup_id = #arg_target_id_lit;
                let desired_id = if prefix.is_empty() {
                    #arg_target_id_lit.to_string()
                } else {
                    format!("{}{}", prefix, #arg_target_id_lit)
                };
                let desired_long = desired_id.replace('.', "-");

                // Find the actual id in the built Command (may differ if clap rewrites it).
                let existing = {
                    let mut iter = cmd.get_arguments();
                    iter
                        .find(|a| a.get_id().as_str() == lookup_id || a.get_long() == Some(#arg_name_lit))
                        .map(|a| a.get_id().as_str().to_string())
                };

                if let Some(existing) = existing {
                    let desired_id_static: &'static str = Box::leak(desired_id.into_boxed_str());
                    let desired_long_static: &'static str = Box::leak(desired_long.into_boxed_str());
                    let existing_static: &'static str = Box::leak(existing.into_boxed_str());
                    cmd = cmd.mut_arg(existing_static, |a| {
                        a.id(::clap::Id::from(desired_id_static))
                            .long(desired_long_static)
                    });
                }
            });

            if !config_meta.skip_toml {
                into_config_setup.push(quote! {
                    let #field_ident = self.#field_ident;
                });

                let init_expr = if base_is_option.is_some() {
                    quote!(#field_ident)
                } else {
                    into_config_required.push(quote! {
                        if #field_ident.is_none() {
                            missing_fields.push(stringify!(#field_ident));
                        }
                    });
                    quote!(#field_ident.unwrap())
                };

                into_config_fields.push(quote! {
                    #field_ident: #init_expr
                });
            }
        }
    }

    let alias_toml_decl = alias_toml_ident
        .as_ref()
        .map(|alias| quote! { #vis type #alias #ty_generics = #toml_ident #ty_generics; })
        .unwrap_or_else(TokenStream2::new);

    let alias_args_decl = alias_args_ident
        .as_ref()
        .map(|alias| quote! { #vis type #alias #ty_generics = #args_ident #ty_generics; })
        .unwrap_or_else(TokenStream2::new);

    // alias from original name if the struct was renamed
    let alias_orig_decl = alias_orig_decl;

    let tokens = quote! {
        #(#attrs)*
        #vis struct #ident #generics #where_clause {
            #(#base_fields)*
        }

        #alias_orig_decl
        #(#attrs)*
        #[derive(Debug, Clone, ::serde::Deserialize, Default)]
        #[serde(default, deny_unknown_fields)]
        #vis struct #toml_ident #generics #where_clause {
            #(#toml_fields)*
        }

        #alias_toml_decl

        #[derive(Debug, Clone, ::clap::Args, Default)]
        struct #derived_args_ident #generics #where_clause {
            #(#derived_args_fields)*
        }

        #(#attrs)*
        #[derive(Debug, Clone, Default)]
        #vis struct #args_ident #generics #where_clause {
            #(#plain_args_fields)*
        }

        #alias_args_decl

        impl #impl_generics #toml_ident #ty_generics #where_clause {
            pub fn merge_with_args(self, args: #args_ident #ty_generics) -> Self {
                Self {
                    #(#merge_fields)*
                }
            }
        }

        impl #impl_generics #args_ident #ty_generics #where_clause {
            pub fn has_overrides(&self) -> bool {
                false #( || #has_override_checks )*
            }

            pub fn namespace_command(mut cmd: clap::Command, prefix: &str) -> clap::Command {
                #( #namespace_steps )*
                let group_name = stringify!(#derived_args_ident);
                cmd = cmd.mut_group(group_name, |_g| ::clap::ArgGroup::new(group_name));
                cmd
            }
        }

        impl #impl_generics ::clap::FromArgMatches for #args_ident #ty_generics #where_clause {
            fn from_arg_matches(matches: &::clap::ArgMatches) -> ::clap::error::Result<Self> {
                let parsed = <#derived_args_ident as ::clap::FromArgMatches>::from_arg_matches(matches)?;
                Ok(Self {
                    #(#args_copy_fields,)*
                })
            }

            fn update_from_arg_matches(
                &mut self,
                matches: &::clap::ArgMatches,
            ) -> ::clap::error::Result<()> {
                let mut parsed = <#derived_args_ident as ::clap::FromArgMatches>::from_arg_matches(matches)?;
                #(#args_update_fields;)*
                Ok(())
            }
        }

        impl #impl_generics ::clap::Args for #args_ident #ty_generics #where_clause {
            fn augment_args(cmd: ::clap::Command) -> ::clap::Command {
                let cmd = <#derived_args_ident as ::clap::Args>::augment_args(cmd);
                Self::namespace_command(cmd, "")
            }

            fn augment_args_for_update(cmd: ::clap::Command) -> ::clap::Command {
                let cmd = <#derived_args_ident as ::clap::Args>::augment_args_for_update(cmd);
                Self::namespace_command(cmd, "")
            }
        }

        impl #impl_generics #toml_ident #ty_generics #where_clause {
            pub fn try_into_config(self) -> anyhow::Result<#ident #ty_generics> {
                let mut missing_fields: Vec<&str> = Vec::new();
                #( #into_config_setup )*
                #( #into_config_required )*

                if !missing_fields.is_empty() {
                    anyhow::bail!("Missing required config fields: {}", missing_fields.join(", "));
                }

                Ok(#ident {
                    #(#into_config_fields,)*
                })
            }
        }

        impl #impl_generics #ident #ty_generics #where_clause {
            pub fn from_sources(
                toml: Option<#toml_ident #ty_generics>,
                args: #args_ident #ty_generics,
            ) -> anyhow::Result<Self> {
                let merged = toml.unwrap_or_default().merge_with_args(args);
                merged.try_into_config()
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
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(lit), ..
                }) = name_value.value
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
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(lit), ..
                }) = name_value.value
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
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(lit), ..
                }) = name_value.value
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
                ));
            }
        }
    }
    Ok(())
}

fn is_arg_meta(meta: &Meta, key: &str) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident(key),
        Meta::NameValue(name_value) => name_value.path.is_ident(key),
        Meta::List(list) => list.path.is_ident(key),
    }
}

fn meta_string_value(metas: &[Meta], key: &str) -> Option<String> {
    for meta in metas {
        if let Meta::NameValue(name_value) = meta {
            if name_value.path.is_ident(key) {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(lit), ..
                }) = &name_value.value
                {
                    return Some(lit.value());
                }
            }
        }
    }
    None
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
        "config_struct fields must use a path type to derive nested names",
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

fn option_inner_type(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if let Some(last) = type_path.path.segments.last() {
            if last.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty.clone());
                    }
                }
            }
        }
    }
    None
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
