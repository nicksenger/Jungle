use proc_macro::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma, Attribute, Data,
    DeriveInput, Expr, Fields, GenericParam, ImplItem, ImplItemFn, ImplItemType, ItemImpl, Meta,
    Path, Type,
};

fn derive_with_properties(input: TokenStream, properties: &[Path]) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    input
        .attrs
        .push(parse_quote!(#[inception(properties = [#(#properties),*])]));
    let generated = inception_derive_gen::State::gen(quote!(#input));
    rewrite_inception_fallback(generated).into()
}

fn derive_with_properties_input(
    mut input: DeriveInput,
    properties: &[Path],
) -> proc_macro2::TokenStream {
    input
        .attrs
        .push(parse_quote!(#[inception(properties = [#(#properties),*])]));
    let generated = inception_derive_gen::State::gen(quote!(#input));
    rewrite_inception_fallback(generated)
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
        "JungleRunning",
        "JungleWaiting",
        "JungleFlow",
        "JungleDynFlow",
        "JungleDynFlowContext",
        "JungleJourneyAst",
        "JungleTraverseFlow",
        "JungleReplaceFlow",
    ]);
    derive_with_properties(input, &properties)
}

#[proc_macro_derive(Flow)]
pub fn derive_flow(input: TokenStream) -> TokenStream {
    let properties = jungle_types(&[
        "JungleRunning",
        "JungleWaiting",
        "JungleFlow",
        "JungleDynFlow",
        "JungleDynFlowContext",
        "JungleJourneyAst",
        "JungleTraverseFlow",
        "JungleReplaceFlow",
    ]);
    derive_with_properties(input, &properties)
}

#[proc_macro_derive(FlowTemplate, attributes(jungle))]
pub fn derive_flow_template(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let view_ty = parse_jungle_view_attr(&input.attrs);
    let scoped_inner_ty = if view_ty.is_some() {
        match &input.data {
            Data::Struct(data) => match &data.fields {
                Fields::Named(named) if named.named.len() == 1 => {
                    named.named.first().map(|f| f.ty.clone())
                }
                Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                    unnamed.unnamed.first().map(|f| f.ty.clone())
                }
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    };

    let properties = if view_ty.is_some() {
        jungle_types(&["JungleFlow", "JungleJourneyAst", "JungleReplaceFlow"])
    } else {
        jungle_types(&[
            "JungleFlow",
            "JungleJourneyAst",
            "JungleTraverseFlow",
            "JungleReplaceFlow",
        ])
    };
    let derived = derive_with_properties_input(input, &properties);
    let template_scope = jungle_type("TemplateScope");
    let root_scope = jungle_type("RootTemplateScope");
    let scope_ty = quote!(#root_scope);
    let scope_impl = quote! {
        impl #impl_generics #template_scope for #ident #ty_generics #where_clause {
            type View = #scope_ty;
        }
    };
    let traverse_flow = jungle_type("TraverseFlow");
    let scoped = jungle_type("Scoped");
    let traverse_impl = if let (Some(view), Some(inner_ty)) = (&view_ty, &scoped_inner_ty) {
        quote! {
            impl #impl_generics #traverse_flow for #ident #ty_generics #where_clause
            where
                #inner_ty: #traverse_flow,
            {
                type Output = #scoped<#view, <#inner_ty as #traverse_flow>::Output>;
            }
        }
    } else if view_ty.is_some() {
        syn::Error::new_spanned(
            &ident,
            "FlowTemplate with `#[jungle(view = ...)]` must be a single-field struct.",
        )
        .to_compile_error()
    } else {
        quote! {}
    };

    quote! {
        #derived
        #scope_impl
        #traverse_impl
    }
    .into()
}

#[proc_macro_derive(Animals)]
pub fn derive_animals(input: TokenStream) -> TokenStream {
    let properties = jungle_types(&["Ident", "JungleAnimals"]);
    derive_with_properties(input, &properties)
}

#[proc_macro_derive(Effects)]
pub fn derive_effects(input: TokenStream) -> TokenStream {
    let properties = jungle_types(&["Ident", "JungleEffects"]);
    derive_with_properties(input, &properties)
}

#[proc_macro_derive(Optic, attributes(view, jungle_sdk))]
pub fn derive_optic(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let properties = jungle_types(&["JungleOptic"]);
    let derived = derive_with_properties_input(input.clone(), &properties);
    let view_project = jungle_type("ViewProject");

    let mut projection_impls = Vec::new();
    if let Data::Struct(data) = &input.data {
        match &data.fields {
            Fields::Named(named) => {
                for field in &named.named {
                    if !is_view_marker(&field.attrs) {
                        continue;
                    }
                    let Some(field_ident) = &field.ident else {
                        continue;
                    };
                    let ty = &field.ty;
                    projection_impls.push(quote! {
                        impl #impl_generics #view_project<#ty> for #ident #ty_generics #where_clause {
                            fn project_view<'a>(state: &'a mut Self) -> &'a mut #ty {
                                &mut state.#field_ident
                            }
                        }
                    });
                }
            }
            Fields::Unnamed(unnamed) => {
                for (index, field) in unnamed.unnamed.iter().enumerate() {
                    if !is_view_marker(&field.attrs) {
                        continue;
                    }
                    let idx = syn::Index::from(index);
                    let ty = &field.ty;
                    projection_impls.push(quote! {
                        impl #impl_generics #view_project<#ty> for #ident #ty_generics #where_clause {
                            fn project_view<'a>(state: &'a mut Self) -> &'a mut #ty {
                                &mut state.#idx
                            }
                        }
                    });
                }
            }
            Fields::Unit => {}
        }
    }

    quote! {
        #derived
        #(#projection_impls)*
    }
    .into()
}

fn is_view_marker(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("view")
            || attr
                .path()
                .segments
                .last()
                .map(|seg| seg.ident == "view")
                .unwrap_or(false)
    })
}

fn parse_jungle_view_attr(attrs: &[Attribute]) -> Option<Type> {
    for attr in attrs {
        if !attr.path().is_ident("jungle") {
            continue;
        }
        let mut result = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("view") {
                let value = meta.value()?;
                let ty: Type = value.parse()?;
                result = Some(ty);
                return Ok(());
            }
            Ok(())
        });
        if result.is_some() {
            return result;
        }
    }
    None
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

#[proc_macro_attribute]
pub fn effect(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_tokens = proc_macro2::TokenStream::from(attr);
    if !attr_tokens.is_empty() {
        return syn::Error::new_spanned(attr_tokens, "This macro does not accept arguments.")
            .to_compile_error()
            .into();
    }

    let item_impl = parse_macro_input!(item as ItemImpl);

    let Some((_, trait_path, _)) = &item_impl.trait_ else {
        return syn::Error::new_spanned(
            &item_impl,
            "This macro can only be applied to trait implementations.",
        )
        .to_compile_error()
        .into();
    };

    let trait_ident = trait_path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
        .unwrap_or_default();
    if trait_ident != "Effect" {
        return syn::Error::new_spanned(trait_path, "Expected an impl for an `Effect<...>` trait.")
            .to_compile_error()
            .into();
    }

    let context_ty = match trait_path
        .segments
        .last()
        .and_then(|segment| match &segment.arguments {
            syn::PathArguments::AngleBracketed(args) if args.args.len() == 1 => {
                match args.args.first() {
                    Some(syn::GenericArgument::Type(ty)) => Some(ty.clone()),
                    _ => None,
                }
            }
            _ => None,
        }) {
        Some(ty) => ty,
        None => {
            return syn::Error::new_spanned(
                trait_path,
                "Expected `Effect<Context>` with exactly one type argument.",
            )
            .to_compile_error()
            .into()
        }
    };

    let mut id_ty: Option<Type> = None;
    let mut in_ty: Option<Type> = None;
    let mut out_ty: Option<Type> = None;
    let mut err_ty: Option<Type> = None;
    let mut effect_fn: Option<ImplItemFn> = None;

    for item in &item_impl.items {
        match item {
            ImplItem::Type(ImplItemType { ident, ty, .. }) if ident == "Id" => {
                id_ty = Some(ty.clone());
            }
            ImplItem::Type(ImplItemType { ident, ty, .. }) if ident == "In" => {
                in_ty = Some(ty.clone());
            }
            ImplItem::Type(ImplItemType { ident, ty, .. }) if ident == "Out" => {
                out_ty = Some(ty.clone());
            }
            ImplItem::Type(ImplItemType { ident, ty, .. }) if ident == "Err" => {
                err_ty = Some(ty.clone());
            }
            ImplItem::Fn(func) if func.sig.ident == "effect" => {
                effect_fn = Some(func.clone());
            }
            _ => {}
        }
    }

    let (Some(id_ty), Some(in_ty), Some(out_ty), Some(err_ty), Some(effect_fn)) =
        (id_ty, in_ty, out_ty, err_ty, effect_fn)
    else {
        return syn::Error::new_spanned(
            &item_impl,
            "Expected associated types `Id`, `In`, `Out`, `Err`, and method `effect`.",
        )
        .to_compile_error()
        .into();
    };

    let self_ty = &item_impl.self_ty;

    let generic_names = item_impl
        .generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Type(ty) => ty.ident.to_string(),
            GenericParam::Lifetime(lifetime) => lifetime.lifetime.ident.to_string(),
            GenericParam::Const(const_param) => const_param.ident.to_string(),
        })
        .collect::<HashSet<_>>();

    let self_ty_names = collect_ident_names(quote!(#self_ty));
    let context_names = collect_ident_names(quote!(#context_ty));
    let self_generic_names = self_ty_names
        .intersection(&generic_names)
        .cloned()
        .collect::<HashSet<_>>();
    let context_only_generic_names = context_names
        .intersection(&generic_names)
        .filter(|name| !self_generic_names.contains(*name))
        .cloned()
        .collect::<HashSet<_>>();

    let schema_types_tokens = quote! { #id_ty #in_ty #out_ty #err_ty };
    let schema_type_names = collect_ident_names(schema_types_tokens);
    if let Some(offender) = context_only_generic_names
        .iter()
        .find(|name| schema_type_names.contains(*name))
    {
        return syn::Error::new_spanned(
            &item_impl,
            format!(
                "Effect schema cannot depend on context-only generic `{offender}`. Move it onto the effect type or make schema context-agnostic."
            ),
        )
        .to_compile_error()
        .into();
    }

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

    let schema_impl_generics = if retained_params.is_empty() {
        quote! {}
    } else {
        quote! { <#(#retained_params),*> }
    };
    let schema_where_predicates = item_impl
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
    let schema_where_clause = if schema_where_predicates.is_empty() {
        quote! {}
    } else {
        quote! { where #(#schema_where_predicates),* }
    };

    let (exec_impl_generics, _, exec_where_clause) = item_impl.generics.split_for_impl();
    let effect_schema = jungle_type("EffectSchema");
    let effect_exec = jungle_type("EffectExec");

    quote! {
        impl #schema_impl_generics #effect_schema for #self_ty #schema_where_clause {
            type Id = #id_ty;
            type In = #in_ty;
            type Out = #out_ty;
            type Err = #err_ty;
        }

        impl #exec_impl_generics #effect_exec<#context_ty> for #self_ty #exec_where_clause {
            #effect_fn
        }
    }
    .into()
}
