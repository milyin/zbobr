use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input, punctuated::Punctuated, Attribute, Fields, GenericArgument, ItemStruct, Lit,
    LitStr, Meta, Token, Type, TypePath,
};

#[proc_macro_attribute]
pub fn config_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "config_struct does not accept arguments",
        )
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
        ident,
        fields,
        generics,
        ..
    } = item;

    // config_struct requires the struct name to end with "Config".
    if !ident.to_string().ends_with("Config") {
        return Err(syn::Error::new_spanned(
            &ident,
            "config_struct requires the struct name to end with 'Config'",
        ));
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
    // Only leaf fields appear in the Derived struct (no flatten for nested).
    let mut derived_args_fields = Vec::new();
    let mut plain_args_fields = Vec::new();
    let mut merge_fields = Vec::new();
    let mut has_override_checks = Vec::new();
    let mut base_fields = Vec::new();
    // Steps that build the struct in PrefixedArgs::from_matches_prefixed (one per field).
    let mut prefixed_from_steps: Vec<TokenStream2> = Vec::new();
    // (heading_prefix_lit, nested_args_ty) pairs used in augment_args_prefixed.
    let mut nested_augment_entries: Vec<(TokenStream2, TokenStream2)> = Vec::new();
    // All field idents in declaration order — used in the Ok(Self { ... }) constructor.
    let mut plain_field_idents: Vec<TokenStream2> = Vec::new();
    let mut into_config_setup = Vec::new();
    let mut into_config_required = Vec::new();
    let mut into_config_fields = Vec::new();
    // Fields for the Config::build impl.
    let mut config_build_fields: Vec<TokenStream2> = Vec::new();

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
        // Track every field ident for the Ok(Self { ... }) constructor.
        plain_field_idents.push(quote!(#field_ident));
        // Reuse doc/other attributes on both structs.
        let args_attrs = other_attrs.clone();
        let mut serde_attrs: Vec<TokenStream2> = Vec::new();
        if let Some(name) = config_meta.toml_rename.as_ref() {
            let lit = LitStr::new(name, Span::call_site());
            serde_attrs.push(quote!(rename = #lit));
        }
        for alias in &config_meta.toml_aliases {
            let lit = LitStr::new(alias, Span::call_site());
            serde_attrs.push(quote!(alias = #lit));
        }
        let rename_attr_tokens = if serde_attrs.is_empty() {
            TokenStream2::new()
        } else {
            quote!(#[serde(#(#serde_attrs),*)])
        };

        let field_snake = field_ident.to_string().to_snake_case();
        let field_kebab = field_snake.replace('_', "-");
        let heading_prefix_value = config_meta
            .heading_prefix
            .clone()
            .unwrap_or_else(|| field_kebab.clone());
        let heading_prefix_lit = config_meta
            .heading_prefix
            .as_ref()
            .map(|value| LitStr::new(value, Span::call_site()))
            .unwrap_or_else(|| LitStr::new(&field_kebab, Span::call_site()));

        let is_nested = config_meta.nested;
        let is_path = config_meta.path;

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

            let heading_text = config_meta
                .help_heading
                .clone()
                .or_else(|| doc_help.clone());
            let _heading = format_help_heading(&heading_prefix_value, heading_text.as_deref());
            let _ = _heading; // was used for next_help_heading; the PrefixedArgs impl uses prefix-derived headings instead

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

            // Nested fields are NOT added to derived_args_fields; they are
            // handled separately in the PrefixedArgs impl below.

            plain_args_fields.push(quote! {
                #(#args_attrs)*
                #field_vis #field_ident: #nested_args_ty,
            });

            has_override_checks.push(quote! {
                self.#field_ident.has_overrides()
            });

            nested_augment_entries.push((quote!(#heading_prefix_lit), quote!(#nested_args_ty)));

            prefixed_from_steps.push(quote! {
                let #field_ident = {
                    let nested_prefix = if prefix.is_empty() {
                        format!("{}.", #heading_prefix_lit)
                    } else {
                        format!("{}{}.", prefix, #heading_prefix_lit)
                    };
                    <#nested_args_ty as ::zbobr_utility::PrefixedArgs>::from_matches_prefixed(__matches, &nested_prefix)?
                };
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

            // For nested fields in Config::build, delegate to the nested Config::build.
            let base_is_option_nested = option_inner_type(&field_ty);
            let suffix_target_ty_build = base_is_option_nested.as_ref().unwrap_or(&field_ty);
            let build_expr = if base_is_option_nested.is_some() {
                // Option<NestedConfig>: only build if TOML section was present
                quote! {
                    #field_ident: __merged.#field_ident.map(|t| {
                        <#suffix_target_ty_build as ::zbobr_api::config::Config>::build(
                            ::std::option::Option::Some(t),
                            ::std::default::Default::default(),
                            config_dir,
                        )
                    })
                }
            } else {
                // Required NestedConfig: always build, passing through merged TOML (may be None)
                quote! {
                    #field_ident: <#suffix_target_ty_build as ::zbobr_api::config::Config>::build(
                        __merged.#field_ident,
                        ::std::default::Default::default(),
                        config_dir,
                    )
                }
            };
            config_build_fields.push(build_expr);
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

            has_override_checks.push(quote! {
                self.#field_ident.is_some()
            });

            // Generate the from_matches_prefixed step for this leaf field.
            // Detect whether value_ty is Vec<T> and use get_many in that case.
            let base_is_option_for_from = option_inner_type(&field_ty);
            let value_ty_for_from = base_is_option_for_from.clone().unwrap_or(field_ty.clone());
            let from_step = if let Some(elem_ty) = vec_inner_type(&value_ty_for_from) {
                quote! {
                    let #field_ident = {
                        let __id = if prefix.is_empty() {
                            #arg_target_id_lit.to_string()
                        } else {
                            format!("{}{}", prefix, #arg_target_id_lit)
                        };
                        __matches.get_many::<#elem_ty>(&__id)
                            .map(|v| v.cloned().collect::<::std::vec::Vec<_>>())
                    };
                }
            } else {
                quote! {
                    let #field_ident = {
                        let __id = if prefix.is_empty() {
                            #arg_target_id_lit.to_string()
                        } else {
                            format!("{}{}", prefix, #arg_target_id_lit)
                        };
                        __matches.get_one::<#value_ty_for_from>(&__id).cloned()
                    };
                }
            };
            prefixed_from_steps.push(from_step);

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

            // Generate Config::build field assignment.
            let build_expr = if is_path {
                if base_is_option.is_some() {
                    // Option<PathBuf>: merged.field.map(|p| resolve_path(p, config_dir))
                    quote! {
                        #field_ident: __merged.#field_ident.map(|p| ::zbobr_utility::resolve_path(p, config_dir))
                    }
                } else if vec_inner_type(&value_ty).is_some() {
                    // Vec<PathBuf>: merged.field.map(|v| ...).unwrap_or(defaults.field)
                    quote! {
                        #field_ident: __merged.#field_ident
                            .map(|v| v.into_iter().map(|p| ::zbobr_utility::resolve_path(p, config_dir)).collect())
                            .unwrap_or(defaults.#field_ident)
                    }
                } else {
                    // PathBuf: merged.field.map(|p| resolve_path(p, config_dir)).unwrap_or(defaults.field)
                    quote! {
                        #field_ident: __merged.#field_ident
                            .map(|p| ::zbobr_utility::resolve_path(p, config_dir))
                            .unwrap_or(defaults.#field_ident)
                    }
                }
            } else if base_is_option.is_some() {
                // Option<T>: merged.field (already Option)
                quote! {
                    #field_ident: __merged.#field_ident
                }
            } else {
                // T: merged.field.unwrap_or(defaults.field)
                quote! {
                    #field_ident: __merged.#field_ident.unwrap_or(defaults.#field_ident)
                }
            };
            config_build_fields.push(build_expr);
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

    // Split the nested augment pairs into two parallel Vecs so quote! can
    // repeat them together in the generated PrefixedArgs impl.
    let (nested_augment_prefixes, nested_augment_tys): (Vec<_>, Vec<_>) =
        nested_augment_entries.into_iter().unzip();

    let tokens = quote! {
        #(#attrs)*
        #vis struct #ident #generics #where_clause {
            #(#base_fields)*
        }

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
        }

        impl #impl_generics ::zbobr_utility::PrefixedArgs for #args_ident #ty_generics #where_clause {
            fn augment_args_prefixed(mut cmd: ::clap::Command, prefix: &str) -> ::clap::Command {
                if prefix.is_empty() {
                    // Augment directly so args inherit the parent command's
                    // current_help_heading (set via next_help_heading on the flatten field).
                    // Going through a temp command would lock help_heading to Some(None),
                    // preventing the parent heading from being applied.
                    cmd = <#derived_args_ident as ::clap::Args>::augment_args(cmd);
                } else {
                    // Build a temporary sub-command from the derived (leaf-only) struct to
                    // obtain correctly-typed clap::Arg definitions.  We then rename every
                    // arg's id and --long form to include the supplied prefix.
                    let __tmp = <#derived_args_ident as ::clap::Args>::augment_args(
                        ::clap::Command::new("__prefixed_tmp")
                    );
                    let __tmp_args: ::std::vec::Vec<::clap::Arg> =
                        __tmp.get_arguments().cloned().collect();
                    for __arg in &__tmp_args {
                        let __base_id = __arg.get_id().as_str();
                        let __new_id: ::std::string::String = format!("{}{}", prefix, __base_id);
                        let __new_long = __new_id.replace('.', "-");
                        let __heading: &'static str = ::std::boxed::Box::leak(
                            format!("[{}]", prefix.trim_end_matches('.')).into_boxed_str(),
                        );
                        let __id_s: &'static str =
                            ::std::boxed::Box::leak(__new_id.into_boxed_str());
                        let __long_s: &'static str =
                            ::std::boxed::Box::leak(__new_long.into_boxed_str());
                        let __new_arg = __arg.clone().id(__id_s).long(__long_s)
                            .help_heading(::std::option::Option::Some(__heading));
                        cmd = cmd.arg(__new_arg);
                    }
                }
                // Recurse into nested structs.
                #(
                    {
                        let __nested_prefix = if prefix.is_empty() {
                            format!("{}.", #nested_augment_prefixes)
                        } else {
                            format!("{}{}.", prefix, #nested_augment_prefixes)
                        };
                        cmd = <#nested_augment_tys as ::zbobr_utility::PrefixedArgs>::augment_args_prefixed(cmd, &__nested_prefix);
                    }
                )*
                cmd
            }

            fn from_matches_prefixed(
                __matches: &::clap::ArgMatches,
                prefix: &str,
            ) -> ::clap::error::Result<Self> {
                #( #prefixed_from_steps )*
                ::std::result::Result::Ok(Self {
                    #(#plain_field_idents,)*
                })
            }
        }

        impl #impl_generics ::clap::FromArgMatches for #args_ident #ty_generics #where_clause {
            fn from_arg_matches(matches: &::clap::ArgMatches) -> ::clap::error::Result<Self> {
                <Self as ::zbobr_utility::PrefixedArgs>::from_matches_prefixed(matches, "")
            }

            fn update_from_arg_matches(
                &mut self,
                matches: &::clap::ArgMatches,
            ) -> ::clap::error::Result<()> {
                *self = <Self as ::zbobr_utility::PrefixedArgs>::from_matches_prefixed(matches, "")?;
                Ok(())
            }
        }

        impl #impl_generics ::clap::Args for #args_ident #ty_generics #where_clause {
            fn augment_args(cmd: ::clap::Command) -> ::clap::Command {
                <Self as ::zbobr_utility::PrefixedArgs>::augment_args_prefixed(cmd, "")
            }

            fn augment_args_for_update(cmd: ::clap::Command) -> ::clap::Command {
                <Self as ::zbobr_utility::PrefixedArgs>::augment_args_prefixed(cmd, "")
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

        impl #impl_generics ::zbobr_api::config::Config for #ident #ty_generics #where_clause {
            type Toml = #toml_ident #ty_generics;
            type Args = #args_ident #ty_generics;
            fn build(toml: ::std::option::Option<Self::Toml>, args: Self::Args, config_dir: &::std::path::Path) -> Self {
                let defaults = Self::default();
                let __merged = toml.unwrap_or_default().merge_with_args(args);
                let _ = &__merged;
                Self {
                    #(#config_build_fields,)*
                }
            }
        }

    };

    Ok(tokens)
}

#[derive(Default)]
struct FieldConfig {
    nested: bool,
    path: bool,
    help_heading: Option<String>,
    skip_toml: bool,
    nested_args_ty: Option<TypePath>,
    nested_toml_ty: Option<TypePath>,
    heading_prefix: Option<String>,
    toml_rename: Option<String>,
    toml_aliases: Vec<String>,
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
            Meta::Path(path) if path.is_ident("path") => config.path = true,
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
            Meta::NameValue(name_value) if name_value.path.is_ident("toml_alias") => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(lit), ..
                }) = name_value.value
                {
                    config.toml_aliases.push(lit.value());
                } else {
                    return Err(syn::Error::new_spanned(
                        name_value,
                        "toml_alias must be a string literal",
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
            let base = last.ident.to_string();
            let stripped = base.strip_suffix("Config").unwrap_or(&base);
            last.ident = format_ident!("{}{}", stripped, suffix);
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

/// If `ty` is `Vec<T>`, return `Some(T)`; otherwise `None`.
fn vec_inner_type(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if let Some(last) = type_path.path.segments.last() {
            if last.ident == "Vec" {
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
    let _ = description;
    let prefix = prefix_kebab.replace('-', ".");
    format!("[{prefix}]")
}
