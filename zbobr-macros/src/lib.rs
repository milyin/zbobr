use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Attribute, Fields, Ident, ItemStruct, LitStr, Meta, Token};

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

    for field in fields_named {
        let field_ident = field.ident.clone().ok_or_else(|| {
            syn::Error::new_spanned(&field, "config_struct! fields must be named")
        })?;
        let field_vis = field.vis.clone();
        let field_ty = field.ty.clone();

        let (other_attrs, arg_metas) = partition_field_attrs(&field.attrs)?;
        // Reuse doc/other attributes on both structs.
        let args_attrs = other_attrs.clone();

        let field_snake = field_ident.to_string().to_snake_case();
        let field_kebab = field_snake.replace('_', "-");

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

        toml_fields.push(quote! {
            #(#other_attrs)*
            #field_vis #field_ident: Option<#field_ty>,
        });

        args_fields.push(quote! {
            #(#args_attrs)*
            #arg_attr
            #field_vis #field_ident: Option<#field_ty>,
        });

        merge_fields.push(quote! {
            #field_ident: args.#field_ident.or(self.#field_ident),
        });
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
    };

    Ok(tokens)
}

fn partition_field_attrs(attrs: &[Attribute]) -> syn::Result<(Vec<Attribute>, Vec<Meta>)> {
    let mut other = Vec::new();
    let mut arg_items = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("arg") {
            let metas: Punctuated<Meta, Token![,]> =
                attr.parse_args_with(Punctuated::parse_terminated)?;
            arg_items.extend(metas.into_iter());
        } else {
            other.push(attr.clone());
        }
    }

    Ok((other, arg_items))
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
