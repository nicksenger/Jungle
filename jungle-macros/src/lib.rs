use proc_macro::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma, DeriveInput, Expr,
    GenericParam, ItemImpl, Meta, Path,
};

fn derive_with_properties(input: TokenStream, properties: &[Path]) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    input
        .attrs
        .push(parse_quote!(#[inception(properties = [#(#properties),*])]));
    let generated = inception_derive_gen::State::gen(quote!(#input));
    rewrite_inception_fallback(generated).into()
}

fn jungle_types_path() -> Path {
    match crate_name("jungle-types") {
        Ok(FoundCrate::Itself) => parse_quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = format_ident!("{name}");
            parse_quote!(::#ident)
        }
        Err(_) => match crate_name("jungle-sdk") {
            Ok(FoundCrate::Itself) => parse_quote!(crate::types),
            Ok(FoundCrate::Name(name)) => {
                let ident = format_ident!("{name}");
                parse_quote!(::#ident::types)
            }
            Err(_) => parse_quote!(jungle_types),
        },
    }
}

fn jungle_type(name: &str) -> Path {
    let types = jungle_types_path();
    let ident = format_ident!("{name}");
    parse_quote!(#types::#ident)
}

fn jungle_types(names: &[&str]) -> Vec<Path> {
    names.iter().map(|name| jungle_type(name)).collect()
}

fn sdk_crate_path() -> Option<proc_macro2::TokenStream> {
    match crate_name("jungle-sdk") {
        Ok(FoundCrate::Itself) => Some(quote!(crate)),
        Ok(FoundCrate::Name(name)) => {
            let ident = format_ident!("{name}");
            Some(quote!(::#ident))
        }
        Err(_) => None,
    }
}

fn inception_path() -> proc_macro2::TokenStream {
    match crate_name("inception") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = format_ident!("{name}");
            quote!(::#ident)
        }
        Err(_) => {
            if let Some(sdk_path) = sdk_crate_path() {
                quote!(#sdk_path::inception)
            } else {
                quote!(::inception)
            }
        }
    }
}

fn rewrite_stream_with_sdk_inception(
    stream: proc_macro2::TokenStream,
    sdk_path: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    use proc_macro2::{Group, TokenTree};

    let mut out = proc_macro2::TokenStream::new();
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut i = 0usize;

    while i < tokens.len() {
        if i + 2 < tokens.len() {
            let a = &tokens[i];
            let b = &tokens[i + 1];
            let c = &tokens[i + 2];

            let leading_colons = matches!(a, TokenTree::Punct(p) if p.as_char() == ':')
                && matches!(b, TokenTree::Punct(p) if p.as_char() == ':');
            let inception_ident = matches!(c, TokenTree::Ident(ident) if ident == "inception");

            if leading_colons && inception_ident {
                out.extend(quote!(#sdk_path::inception));
                i += 3;
                continue;
            }
        }

        match &tokens[i] {
            TokenTree::Group(group) => {
                let mut next = Group::new(
                    group.delimiter(),
                    rewrite_stream_with_sdk_inception(group.stream(), sdk_path),
                );
                next.set_span(group.span());
                out.extend(std::iter::once(TokenTree::Group(next)));
            }
            _ => out.extend(std::iter::once(tokens[i].clone())),
        }
        i += 1;
    }

    out
}

fn rewrite_inception_fallback(stream: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    // inception-derive-gen emits ::inception when the downstream crate does not
    // directly depend on inception. In that case, route through jungle-sdk.
    if crate_name("inception").is_ok() {
        return stream;
    }

    let Some(sdk_path) = sdk_crate_path() else {
        return stream;
    };

    rewrite_stream_with_sdk_inception(stream, &sdk_path)
}

#[proc_macro_derive(Journey)]
pub fn derive_journey(input: TokenStream) -> TokenStream {
    let properties = jungle_types(&[
        "JungleFlow",
        "JungleDynFlow",
        "JungleJourneyAst",
        "JungleTraverseFlow",
        "JungleReplaceFlow",
    ]);
    derive_with_properties(input, &properties)
}

#[proc_macro_derive(Flow)]
pub fn derive_flow(input: TokenStream) -> TokenStream {
    let properties = jungle_types(&[
        "JungleFlow",
        "JungleDynFlow",
        "JungleJourneyAst",
        "JungleTraverseFlow",
        "JungleReplaceFlow",
    ]);
    derive_with_properties(input, &properties)
}

#[proc_macro_derive(Animals)]
pub fn derive_animals(input: TokenStream) -> TokenStream {
    let properties = jungle_types(&["Ident", "JungleAnimals"]);
    derive_with_properties(input, &properties)
}

#[proc_macro_derive(Actions)]
pub fn derive_actions(input: TokenStream) -> TokenStream {
    let properties = jungle_types(&["Ident", "JungleActions"]);
    derive_with_properties(input, &properties)
}

#[proc_macro_derive(Optic)]
pub fn derive_optic(input: TokenStream) -> TokenStream {
    let properties = jungle_types(&["JungleOptic"]);
    derive_with_properties(input, &properties)
}

struct PrimitiveAttributes {
    property: Path,
}

impl Parse for PrimitiveAttributes {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let metas = Punctuated::<Meta, Comma>::parse_terminated(input)?;
        let mut property = None;

        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("property") => {
                    if property.is_some() {
                        return Err(syn::Error::new_spanned(
                            nv.path,
                            "Duplicate `property` setting.",
                        ));
                    }
                    let Expr::Path(path_expr) = nv.value else {
                        return Err(syn::Error::new_spanned(
                            nv.value,
                            "Expected `property` to be a path.",
                        ));
                    };
                    property = Some(path_expr.path);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "Unknown `primitive` setting.",
                    ));
                }
            }
        }

        let Some(property) = property else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Missing `property = ...`.",
            ));
        };

        Ok(Self { property })
    }
}

fn collect_ident_names(tokens: proc_macro2::TokenStream) -> HashSet<String> {
    fn walk(stream: proc_macro2::TokenStream, names: &mut HashSet<String>) {
        for tt in stream {
            match tt {
                proc_macro2::TokenTree::Ident(id) => {
                    names.insert(id.to_string());
                }
                proc_macro2::TokenTree::Group(group) => walk(group.stream(), names),
                proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => {}
            }
        }
    }

    let mut names = HashSet::new();
    walk(tokens, &mut names);
    names
}

#[proc_macro_attribute]
pub fn sdk_primitive(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::Item);
    let syn::Item::Impl(item_impl) = input else {
        return syn::Error::new_spanned(
            input,
            "This macro can only be applied to trait implementations.",
        )
        .to_compile_error()
        .into();
    };

    let ItemImpl {
        trait_: Some(_),
        self_ty,
        ..
    } = &item_impl
    else {
        return syn::Error::new_spanned(
            item_impl,
            "This macro can only be applied to trait implementations.",
        )
        .to_compile_error()
        .into();
    };

    let PrimitiveAttributes { property } = match syn::parse(attr) {
        Ok(attrs) => attrs,
        Err(err) => return err.into_compile_error().into(),
    };

    let self_ty_tokens = quote!(#self_ty).to_string();
    let retained_params = item_impl
        .generics
        .params
        .iter()
        .filter(|param| match param {
            GenericParam::Type(ty) => self_ty_tokens.contains(&ty.ident.to_string()),
            GenericParam::Lifetime(lifetime) => {
                self_ty_tokens.contains(&lifetime.lifetime.ident.to_string())
            }
            GenericParam::Const(const_param) => {
                self_ty_tokens.contains(&const_param.ident.to_string())
            }
        })
        .cloned()
        .collect::<Vec<_>>();
    let retained_param_names = retained_params
        .iter()
        .map(|param| match param {
            GenericParam::Type(ty) => ty.ident.to_string(),
            GenericParam::Lifetime(lifetime) => lifetime.lifetime.ident.to_string(),
            GenericParam::Const(const_param) => const_param.ident.to_string(),
        })
        .collect::<HashSet<_>>();
    let dropped_param_names = item_impl
        .generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Type(ty) => ty.ident.to_string(),
            GenericParam::Lifetime(lifetime) => lifetime.lifetime.ident.to_string(),
            GenericParam::Const(const_param) => const_param.ident.to_string(),
        })
        .filter(|name| !retained_param_names.contains(name))
        .collect::<HashSet<_>>();
    let impl_generics = if retained_params.is_empty() {
        quote! {}
    } else {
        quote! { <#(#retained_params),*> }
    };
    let retained_where_predicates = item_impl
        .generics
        .where_clause
        .as_ref()
        .map(|where_clause| {
            where_clause
                .predicates
                .iter()
                .filter(|pred| {
                    let used_names = collect_ident_names(quote!(#pred));
                    !used_names
                        .iter()
                        .any(|name| dropped_param_names.contains(name))
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let impl_where_clause = if retained_where_predicates.is_empty() {
        quote! {}
    } else {
        quote! { where #(#retained_where_predicates),* }
    };

    let inception = inception_path();
    quote! {
        #item_impl
        const _: () = {
            impl #impl_generics #inception::IsPrimitive<#property> for #self_ty #impl_where_clause {
                type Is = #inception::True;
            }
        };
    }
    .into()
}
