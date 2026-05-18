use proc_macro::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma, Attribute, Data,
    DeriveInput, Expr, Fields, FnArg, GenericParam, ImplItem, ImplItemFn, ImplItemType, ItemImpl,
    Lit, Meta, Path, Token, Type, TypeReference, Visibility,
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

fn typosaurus_path() -> proc_macro2::TokenStream {
    match crate_name("typosaurus") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = format_ident!("{name}");
            quote!(::#ident)
        }
        Err(_) => {
            if let Some(sdk_path) = sdk_crate_path() {
                quote!(#sdk_path::typosaurus)
            } else {
                quote!(::typosaurus)
            }
        }
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

fn typenum_const_type(
    typosaurus: &proc_macro2::TokenStream,
    index: usize,
) -> proc_macro2::TokenStream {
    let ident = format_ident!("U{index}");
    quote!(#typosaurus::num::consts::#ident)
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

#[proc_macro_derive(Flow, attributes(jungle))]
pub fn derive_flow(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let data = input.data.clone();
    let ident = input.ident.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let focus_ty = parse_jungle_focus_attr(&input.attrs);
    // Scoped templates provide a custom `TraverseFlow` impl (wrapping output in `Scoped`),
    // so they must not also derive `JungleTraverseFlow` via inception.
    let properties = if focus_ty.is_some() {
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
    let template_scope = jungle_type("FlowScope");
    let root_scope = jungle_type("RootFlowScope");
    let template_view = jungle_type("FlowView");
    let scope_ty = if let Some(focus) = &focus_ty {
        quote!(#template_view<#focus>)
    } else {
        quote!(#root_scope)
    };
    let scope_impl = quote! {
        impl #impl_generics #template_scope for #ident #ty_generics #where_clause {
            type View = #scope_ty;
        }
    };
    let traverse_flow = jungle_type("TraverseFlow");
    let scoped = jungle_type("Scoped");
    let list_empty: Path = parse_quote!(jungle_sdk::typosaurus::collections::list::Empty);
    let field_types = match &data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => named
                .named
                .iter()
                .map(|field| {
                    let ty = &field.ty;
                    quote!(#ty)
                })
                .collect::<Vec<_>>(),
            Fields::Unnamed(unnamed) => unnamed
                .unnamed
                .iter()
                .map(|field| {
                    let ty = &field.ty;
                    quote!(#ty)
                })
                .collect::<Vec<_>>(),
            Fields::Unit => Vec::new(),
        },
        _ => Vec::new(),
    };
    let field_traverse_outputs = field_types
        .iter()
        .map(|ty| quote!(<#ty as #traverse_flow>::Output))
        .collect::<Vec<_>>();
    let scoped_inner = nested_tlist(&field_traverse_outputs, &list_empty);
    let traverse_impl = if let Some(focus) = &focus_ty {
        quote! {
            impl #impl_generics #traverse_flow for #ident #ty_generics #where_clause
            where
                #(#field_types: #traverse_flow,)*
            {
                type Output = #scoped<#focus, #scoped_inner>;
            }
        }
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

#[proc_macro_derive(Optic, attributes(focus, jungle_sdk))]
pub fn derive_optic(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let properties = jungle_types(&["JungleOptic"]);
    let derived = derive_with_properties_input(input.clone(), &properties);
    let view_project = jungle_type("ViewProject");
    let lens_index = jungle_type("LensIndex");
    let typosaurus = typosaurus_path();

    let mut projection_impls = Vec::new();
    let mut lens_index_impls = Vec::new();
    if let Data::Struct(data) = &input.data {
        match &data.fields {
            Fields::Named(named) => {
                for (index, field) in named.named.iter().enumerate() {
                    let Some(field_ident) = &field.ident else {
                        continue;
                    };
                    let ty = &field.ty;
                    let index_ty = typenum_const_type(&typosaurus, index);
                    lens_index_impls.push(quote! {
                        impl #impl_generics #lens_index<#index_ty> for #ident #ty_generics #where_clause {
                            type View = #ty;

                            fn lens_index<'a>(state: &'a mut Self) -> &'a mut #ty {
                                &mut state.#field_ident
                            }
                        }
                    });
                    if !is_focus_marker(&field.attrs) {
                        continue;
                    }
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
                    let idx = syn::Index::from(index);
                    let ty = &field.ty;
                    let index_ty = typenum_const_type(&typosaurus, index);
                    lens_index_impls.push(quote! {
                        impl #impl_generics #lens_index<#index_ty> for #ident #ty_generics #where_clause {
                            type View = #ty;

                            fn lens_index<'a>(state: &'a mut Self) -> &'a mut #ty {
                                &mut state.#idx
                            }
                        }
                    });
                    if !is_focus_marker(&field.attrs) {
                        continue;
                    }
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
        #(#lens_index_impls)*
    }
    .into()
}

fn is_focus_marker(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("focus")
            || attr
                .path()
                .segments
                .last()
                .map(|seg| seg.ident == "focus")
                .unwrap_or(false)
    })
}

fn parse_jungle_focus_attr(attrs: &[Attribute]) -> Option<Type> {
    for attr in attrs {
        if !attr.path().is_ident("jungle") {
            continue;
        }
        let mut result = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("focus") {
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

fn nested_tlist(items: &[proc_macro2::TokenStream], empty: &Path) -> proc_macro2::TokenStream {
    if items.is_empty() {
        return quote!(#empty);
    }
    let head = &items[0];
    let tail = nested_tlist(&items[1..], empty);
    quote!(jungle_sdk::typosaurus::collections::list::List<(#head, #tail)>)
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

struct AnimalAttributes {
    observe: bool,
    perturb: bool,
    id: Option<usize>,
    generation: Option<usize>,
}

impl Parse for AnimalAttributes {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let mut observe = false;
        let mut perturb = false;
        let mut id = None;
        let mut generation = None;

        if input.is_empty() {
            return Ok(Self {
                observe,
                perturb,
                id,
                generation,
            });
        }

        let metas = Punctuated::<Meta, Comma>::parse_terminated(input)?;
        for meta in metas {
            match meta {
                Meta::Path(path) if path.is_ident("observe") => observe = true,
                Meta::Path(path) if path.is_ident("perturb") => perturb = true,
                Meta::NameValue(nv) if nv.path.is_ident("id") => {
                    if id.is_some() {
                        return Err(syn::Error::new_spanned(nv.path, "Duplicate `id` setting."));
                    }
                    id = Some(parse_usize_expr(&nv.value, "id")?);
                }
                Meta::NameValue(nv) if nv.path.is_ident("generation") => {
                    if generation.is_some() {
                        return Err(syn::Error::new_spanned(
                            nv.path,
                            "Duplicate `generation` setting.",
                        ));
                    }
                    generation = Some(parse_usize_expr(&nv.value, "generation")?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "Unknown `animal` setting. Supported: `observe`, `perturb`, `id = ...`, `generation = ...`.",
                    ))
                }
            }
        }

        Ok(Self {
            observe,
            perturb,
            id,
            generation,
        })
    }
}

struct EffectAttributes {
    id: Option<usize>,
}

impl Parse for EffectAttributes {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        if input.is_empty() {
            return Ok(Self { id: None });
        }

        let metas = Punctuated::<Meta, Comma>::parse_terminated(input)?;
        let mut id = None;

        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("id") => {
                    if id.is_some() {
                        return Err(syn::Error::new_spanned(nv.path, "Duplicate `id` setting."));
                    }
                    id = Some(parse_usize_expr(&nv.value, "id")?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "Unknown `effect` setting. Supported: `id = ...`.",
                    ));
                }
            }
        }

        Ok(Self { id })
    }
}

struct ActAttributes {
    aspect: Option<Type>,
    bind: Option<Type>,
}

impl Parse for ActAttributes {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        if input.is_empty() {
            return Ok(Self {
                aspect: None,
                bind: None,
            });
        }

        let mut aspect = None;
        let mut bind = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            if key == "aspect" {
                input.parse::<Token![=]>()?;
                if aspect.is_some() {
                    return Err(syn::Error::new_spanned(key, "Duplicate `aspect` setting."));
                }
                aspect = Some(input.parse::<Type>()?);
            } else if key == "bind" {
                input.parse::<Token![=]>()?;
                if bind.is_some() {
                    return Err(syn::Error::new_spanned(key, "Duplicate `bind` setting."));
                }
                bind = Some(input.parse::<Type>()?);
            } else if key == "bind_vis" {
                // Back-compat: consume and ignore deprecated `bind_vis`.
                input.parse::<Token![=]>()?;
                let _ = input.parse::<Visibility>()?;
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "Unknown `act` setting. Supported: `aspect = ...`, `bind = ...`.",
                ));
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                if input.is_empty() {
                    break;
                }
            } else if !input.is_empty() {
                return Err(input.error("Expected `,` between `act` settings."));
            }
        }

        Ok(Self {
            aspect,
            bind,
        })
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

fn parse_usize_expr(expr: &Expr, field_name: &str) -> Result<usize, syn::Error> {
    let Expr::Lit(expr_lit) = expr else {
        return Err(syn::Error::new_spanned(
            expr,
            format!("`{field_name}` must be an integer literal."),
        ));
    };
    let Lit::Int(value) = &expr_lit.lit else {
        return Err(syn::Error::new_spanned(
            expr,
            format!("`{field_name}` must be an integer literal."),
        ));
    };
    value.base10_parse::<usize>().map_err(|_| {
        syn::Error::new_spanned(
            value,
            format!("`{field_name}` must fit in usize and be non-negative."),
        )
    })
}

fn self_impl_generics(
    item_impl: &ItemImpl,
    self_ty: &Type,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
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
    (impl_generics, impl_where_clause)
}

fn primitive_marker_impl(
    item_impl: &ItemImpl,
    self_ty: &Type,
    property: &Path,
) -> proc_macro2::TokenStream {
    let (impl_generics, impl_where_clause) = self_impl_generics(item_impl, self_ty);
    let inception = inception_path();
    quote! {
        const _: () = {
            impl #impl_generics #inception::IsPrimitive<#property> for #self_ty #impl_where_clause {
                type Is = #inception::True;
            }
        };
    }
}

fn id_inner_from_meta_id(id_ty: &Type) -> Result<Type, syn::Error> {
    let Type::Path(id_ty_path) = id_ty else {
        return Err(syn::Error::new_spanned(
            id_ty,
            "Expected `type Id = Id<U...>;`.",
        ));
    };

    let Some(last) = id_ty_path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            id_ty,
            "Expected `type Id = Id<U...>;`.",
        ));
    };

    if last.ident != "Id" {
        return Err(syn::Error::new_spanned(
            id_ty,
            "Expected `type Id = Id<U...>;`.",
        ));
    }

    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return Err(syn::Error::new_spanned(
            id_ty,
            "Expected `type Id = Id<U...>;`.",
        ));
    };

    if args.args.len() != 1 {
        return Err(syn::Error::new_spanned(
            id_ty,
            "Expected `type Id = Id<U...>;`.",
        ));
    }

    let Some(syn::GenericArgument::Type(inner)) = args.args.first() else {
        return Err(syn::Error::new_spanned(
            id_ty,
            "Expected `type Id = Id<U...>;`.",
        ));
    };

    Ok(inner.clone())
}

fn require_trait_impl(
    item_impl: &ItemImpl,
    trait_name: &str,
    macro_name: &str,
) -> Result<(), syn::Error> {
    let Some((_, trait_path, _)) = &item_impl.trait_ else {
        return Err(syn::Error::new_spanned(
            item_impl,
            format!("`#[{macro_name}]` can only be applied to trait impls."),
        ));
    };

    let Some(segment) = trait_path.segments.last() else {
        return Err(syn::Error::new_spanned(
            trait_path,
            format!("Unable to resolve target trait for `#[{macro_name}]`."),
        ));
    };

    if segment.ident != trait_name {
        return Err(syn::Error::new_spanned(
            trait_path,
            format!("`#[{macro_name}]` must be applied to `impl {trait_name} for ...` blocks."),
        ));
    }

    Ok(())
}

fn view_ty_from_bound_method(
    func: &ImplItemFn,
    method_name: &str,
    expect_mut: bool,
) -> Result<Type, syn::Error> {
    if func.sig.inputs.len() != 2 {
        return Err(syn::Error::new_spanned(
            &func.sig,
            format!("`{method_name}` must accept exactly two arguments."),
        ));
    }

    let Some(FnArg::Typed(first_arg)) = func.sig.inputs.first() else {
        return Err(syn::Error::new_spanned(
            &func.sig,
            format!("`{method_name}` must use typed arguments."),
        ));
    };

    let Type::Reference(TypeReference {
        mutability, elem, ..
    }) = first_arg.ty.as_ref()
    else {
        return Err(syn::Error::new_spanned(
            &first_arg.ty,
            format!("First `{method_name}` argument must be a reference."),
        ));
    };

    if expect_mut && mutability.is_none() {
        return Err(syn::Error::new_spanned(
            &first_arg.ty,
            format!("First `{method_name}` argument must be `&mut`."),
        ));
    }
    if !expect_mut && mutability.is_some() {
        return Err(syn::Error::new_spanned(
            &first_arg.ty,
            format!("First `{method_name}` argument must be `&` (not `&mut`)."),
        ));
    }

    Ok((**elem).clone())
}

fn type_tokens_match(a: &Type, b: &Type) -> bool {
    quote!(#a).to_string() == quote!(#b).to_string()
}

fn self_type_ident(self_ty: &Type) -> Result<syn::Ident, syn::Error> {
    let Type::Path(tp) = self_ty else {
        return Err(syn::Error::new_spanned(
            self_ty,
            "`#[act]` requires a concrete type path for `impl Act for ...`.",
        ));
    };

    let Some(last) = tp.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            self_ty,
            "Unable to determine type name for `#[act]`.",
        ));
    };

    Ok(last.ident.clone())
}

fn id_inner_from_impl(
    item_impl: &ItemImpl,
    macro_name: &str,
    require_non_generic_impl: bool,
) -> Result<Type, syn::Error> {
    if require_non_generic_impl && !item_impl.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item_impl.generics,
            format!("`#[{macro_name}]` currently supports only non-generic impl blocks."),
        ));
    }

    let Some(id_assoc) = item_impl.items.iter().find_map(|item| {
        let ImplItem::Type(ty) = item else {
            return None;
        };
        (ty.ident == "Id").then_some(ty)
    }) else {
        return Err(syn::Error::new_spanned(
            item_impl,
            "Missing associated type `Id`.",
        ));
    };

    id_inner_from_meta_id(&id_assoc.ty)
}

fn emit_identified_animals(
    item_impl: &ItemImpl,
    self_ty: &Type,
    id_inner: &Type,
) -> proc_macro2::TokenStream {
    let types = jungle_types_path();
    let typosaurus = typosaurus_path();
    let node_ty = quote!(#typosaurus::collections::sp::Node<#id_inner, #self_ty>);
    let (impl_generics, impl_where_clause) = self_impl_generics(item_impl, self_ty);

    let animals_prop = jungle_type("JungleAnimals");
    let ident_prop = jungle_type("Ident");
    let animals_marker = primitive_marker_impl(item_impl, self_ty, &animals_prop);
    let identified_marker = primitive_marker_impl(item_impl, self_ty, &ident_prop);

    quote! {
        impl #impl_generics #types::Animals for #self_ty #impl_where_clause {
            type List = #node_ty;
        }
        #animals_marker

        impl #impl_generics #types::Identified for #self_ty #impl_where_clause {
            type Id = #id_inner;
        }
        #identified_marker
    }
}

#[proc_macro_attribute]
pub fn animal(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = match syn::parse::<AnimalAttributes>(attr) {
        Ok(attrs) => attrs,
        Err(err) => return err.into_compile_error().into(),
    };

    let mut item_impl = parse_macro_input!(item as ItemImpl);
    if let Err(err) = require_trait_impl(&item_impl, "Animal", "animal") {
        return err.into_compile_error().into();
    }

    let has_id_assoc = item_impl
        .items
        .iter()
        .any(|item| matches!(item, ImplItem::Type(ty) if ty.ident == "Id"));
    let has_generation_assoc = item_impl
        .items
        .iter()
        .any(|item| matches!(item, ImplItem::Type(ty) if ty.ident == "Generation"));

    if attrs.id.is_some() && has_id_assoc {
        return syn::Error::new_spanned(
            &item_impl,
            "`#[animal(id = ...)]` conflicts with manual `type Id = ...;`.",
        )
        .to_compile_error()
        .into();
    }
    if attrs.generation.is_some() && has_generation_assoc {
        return syn::Error::new_spanned(
            &item_impl,
            "`#[animal(generation = ...)]` conflicts with manual `type Generation = ...;`.",
        )
        .to_compile_error()
        .into();
    }

    let types = jungle_types_path();
    let typosaurus = typosaurus_path();
    if let Some(id) = attrs.id {
        let u_ty = typenum_const_type(&typosaurus, id);
        let generated_id: ImplItem = parse_quote! {
            type Id = #types::Id<#u_ty>;
        };
        item_impl.items.insert(0, generated_id);
    }
    if let Some(generation) = attrs.generation {
        let u_ty = typenum_const_type(&typosaurus, generation);
        let generated_generation: ImplItem = parse_quote! {
            type Generation = #u_ty;
        };
        item_impl.items.insert(0, generated_generation);
    }

    let self_ty = &item_impl.self_ty;
    let id_inner = match id_inner_from_impl(&item_impl, "animal", false) {
        Ok(id) => id,
        Err(err) => return err.into_compile_error().into(),
    };
    let primitives = emit_identified_animals(&item_impl, self_ty, &id_inner);

    let (impl_generics, impl_where_clause) = self_impl_generics(&item_impl, self_ty);
    let observation_ty = if attrs.observe {
        quote!(#types::ObserveObservation)
    } else {
        quote!(#types::NoopObservation)
    };
    let perturbation_ty = if attrs.perturb {
        quote!(#types::TraitPerturbation)
    } else {
        quote!(#types::NoopPerturbation)
    };

    quote! {
        #item_impl

        impl #impl_generics #types::Observable for #self_ty #impl_where_clause {
            type Observation = #observation_ty;
        }

        impl #impl_generics #types::Perturbable for #self_ty #impl_where_clause {
            type Perturbation = #perturbation_ty;
        }

        #primitives
    }
    .into()
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

    let marker = primitive_marker_impl(&item_impl, self_ty, &property);
    quote! {
        #item_impl
        #marker
    }
    .into()
}

#[proc_macro_attribute]
pub fn effect(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = match syn::parse::<EffectAttributes>(attr) {
        Ok(attrs) => attrs,
        Err(err) => return err.into_compile_error().into(),
    };

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

    if attrs.id.is_some() && id_ty.is_some() {
        return syn::Error::new_spanned(
            &item_impl,
            "`#[effect(id = ...)]` conflicts with manual `type Id = ...;`.",
        )
        .to_compile_error()
        .into();
    }

    let types = jungle_types_path();
    let typosaurus = typosaurus_path();
    if id_ty.is_none() {
        if let Some(id) = attrs.id {
            let u_ty = typenum_const_type(&typosaurus, id);
            id_ty = Some(parse_quote!(#types::Id<#u_ty>));
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

    let id_inner = match id_inner_from_meta_id(&id_ty) {
        Ok(id) => id,
        Err(err) => return err.to_compile_error().into(),
    };
    let (schema_impl_generics, schema_where_clause) = self_impl_generics(&item_impl, self_ty);

    let (exec_impl_generics, _, exec_where_clause) = item_impl.generics.split_for_impl();
    let node_ty = quote!(#typosaurus::collections::sp::Node<#id_inner, #self_ty>);
    let effect_schema = jungle_type("EffectSchema");
    let effect_exec = jungle_type("EffectExec");
    let effects_prop = jungle_type("JungleEffects");
    let ident_prop = jungle_type("Ident");
    let effects_marker = primitive_marker_impl(&item_impl, self_ty, &effects_prop);
    let identified_marker = primitive_marker_impl(&item_impl, self_ty, &ident_prop);

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

        impl #schema_impl_generics #types::Effects for #self_ty #schema_where_clause {
            type List = #node_ty;
        }
        #effects_marker

        impl #schema_impl_generics #types::Identified for #self_ty #schema_where_clause {
            type Id = #id_inner;
        }
        #identified_marker
    }
    .into()
}

#[proc_macro_attribute]
pub fn act(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = match syn::parse::<ActAttributes>(attr) {
        Ok(attrs) => attrs,
        Err(err) => return err.into_compile_error().into(),
    };

    let item_impl = parse_macro_input!(item as ItemImpl);
    if let Err(err) = require_trait_impl(&item_impl, "Act", "act") {
        return err.into_compile_error().into();
    }

    if !item_impl.generics.params.is_empty() {
        return syn::Error::new_spanned(
            &item_impl.generics,
            "`#[act]` currently supports only non-generic impl blocks.",
        )
        .to_compile_error()
        .into();
    }

    let mut effect_assoc: Option<ImplItemType> = None;
    let mut input_assoc: Option<ImplItemType> = None;
    let mut output_assoc: Option<ImplItemType> = None;
    let mut emit_fn: Option<ImplItemFn> = None;
    let mut absorb_fn: Option<ImplItemFn> = None;

    for item in &item_impl.items {
        match item {
            ImplItem::Type(ty) if ty.ident == "Effect" => effect_assoc = Some(ty.clone()),
            ImplItem::Type(ty) if ty.ident == "Input" => input_assoc = Some(ty.clone()),
            ImplItem::Type(ty) if ty.ident == "Output" => output_assoc = Some(ty.clone()),
            ImplItem::Type(ty) if ty.ident == "Bind" => {
                return syn::Error::new_spanned(
                    ty,
                    "`#[act]` generates `type Bind<...>`; remove manual `Bind` from this impl.",
                )
                .to_compile_error()
                .into();
            }
            ImplItem::Fn(func) if func.sig.ident == "emit" => emit_fn = Some(func.clone()),
            ImplItem::Fn(func) if func.sig.ident == "absorb" => absorb_fn = Some(func.clone()),
            _ => {}
        }
    }

    let (Some(effect_assoc), Some(input_assoc), Some(output_assoc)) =
        (effect_assoc, input_assoc, output_assoc)
    else {
        return syn::Error::new_spanned(
            &item_impl,
            "Expected `type Effect`, `type Input`, and `type Output`.",
        )
        .to_compile_error()
        .into();
    };

    let self_ty = &item_impl.self_ty;
    let self_ident = match self_type_ident(self_ty) {
        Ok(ident) => ident,
        Err(err) => return err.into_compile_error().into(),
    };
    let bound_ident = format_ident!("__JungleActBound{self_ident}");
    let types = jungle_types_path();
    let default_aspect: Type = parse_quote!(#types::Identity);
    let explicit_bind_ty = attrs.bind.clone();
    if explicit_bind_ty.is_some() && attrs.aspect.is_some() {
        return syn::Error::new_spanned(
            &item_impl.self_ty,
            "`#[act(bind = ...)]` cannot be combined with `aspect = ...`; define the aspect on the explicit bound type instead.",
        )
        .to_compile_error()
        .into();
    }
    let aspect_ty = attrs.aspect.unwrap_or(default_aspect);
    let bind_assoc: ImplItem = if let Some(bind_ty) = explicit_bind_ty.clone() {
        parse_quote! {
            type Bind<A: #types::Animal> = #bind_ty;
        }
    } else {
        parse_quote! {
            type Bind<A: #types::Animal> = #bound_ident<A>;
        }
    };
    let effect_ty = effect_assoc.ty.clone();
    let input_ty = input_assoc.ty.clone();
    let output_ty = output_assoc.ty.clone();

    let mut generated_act_impl = item_impl.clone();
    generated_act_impl.items = vec![
        ImplItem::Type(effect_assoc.clone()),
        ImplItem::Type(input_assoc.clone()),
        ImplItem::Type(output_assoc.clone()),
        bind_assoc,
    ];

    if explicit_bind_ty.is_some() {
        quote! {
            #generated_act_impl
        }
        .into()
    } else {
        let (Some(emit_fn), Some(absorb_fn)) = (emit_fn, absorb_fn) else {
            return syn::Error::new_spanned(
                &item_impl,
                "Expected methods `emit` + `absorb` unless `bind = ...` is provided.",
            )
            .to_compile_error()
            .into();
        };
        let emit_view_ty = match view_ty_from_bound_method(&emit_fn, "emit", false) {
            Ok(ty) => ty,
            Err(err) => return err.to_compile_error().into(),
        };
        let absorb_view_ty = match view_ty_from_bound_method(&absorb_fn, "absorb", true) {
            Ok(ty) => ty,
            Err(err) => return err.to_compile_error().into(),
        };
        if !type_tokens_match(&emit_view_ty, &absorb_view_ty) {
            return syn::Error::new_spanned(
                &absorb_fn.sig,
                "`emit` and `absorb` must operate on the same view type.",
            )
            .to_compile_error()
            .into();
        }

        quote! {
            #generated_act_impl

            pub struct #bound_ident<A>(::core::marker::PhantomData<fn() -> A>);

            impl<A> #types::BoundAct<A> for #bound_ident<A>
            where
                A: #types::Animal,
                #aspect_ty: #types::Aspect<<A as #types::Animal>::State, View = #emit_view_ty>,
            {
                type Effect = #effect_ty;
                type Aspect = #aspect_ty;
                type Input = #input_ty;
                type Output = #output_ty;

                #emit_fn
                #absorb_fn
            }
        }
        .into()
    }
}
