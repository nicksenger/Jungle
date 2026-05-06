use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::Parser, parse_macro_input, parse_quote, punctuated::Punctuated, DeriveInput, Item,
    Path,
};

#[proc_macro]
pub fn noop(input: TokenStream) -> TokenStream {
    input
}

fn derive_with_properties(input: TokenStream, properties: &[Path]) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let impl_params = input.generics.params.iter().cloned().collect::<Vec<_>>();
    let where_preds = input
        .generics
        .where_clause
        .as_ref()
        .map(|wc| wc.predicates.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let (_, ty_generics, _) = input.generics.split_for_impl();

    quote! {
        inception::inception_opt_in_register!(impl [#(#impl_params),*] #name #ty_generics where [#(#where_preds),*] : [#(#properties),*]);
    }
    .into()
}

#[proc_macro_derive(Instinct)]
pub fn derive_instinct(input: TokenStream) -> TokenStream {
    derive_with_properties(
        input,
        &[
            parse_quote!(jungle_types::JungleFlow),
            parse_quote!(jungle_types::JungleDynFlow),
        ],
    )
}

#[proc_macro_derive(Flow)]
pub fn derive_flow(input: TokenStream) -> TokenStream {
    derive_with_properties(input, &[parse_quote!(jungle_types::JungleFlow)])
}

#[proc_macro_derive(Creatures)]
pub fn derive_creatures(input: TokenStream) -> TokenStream {
    derive_with_properties(
        input,
        &[
            parse_quote!(jungle_types::Ident),
            parse_quote!(jungle_types::JungleCreatures),
        ],
    )
}

#[proc_macro_derive(Actions)]
pub fn derive_actions(input: TokenStream) -> TokenStream {
    derive_with_properties(
        input,
        &[
            parse_quote!(jungle_types::Ident),
            parse_quote!(jungle_types::JungleActions),
        ],
    )
}

fn attrs_mut(item: &mut Item) -> Result<&mut Vec<syn::Attribute>, syn::Error> {
    match item {
        Item::Struct(item) => Ok(&mut item.attrs),
        Item::Enum(item) => Ok(&mut item.attrs),
        Item::Union(item) => Ok(&mut item.attrs),
        _ => Err(syn::Error::new_spanned(
            item,
            "jungle macro attributes are supported only on struct/enum/union items",
        )),
    }
}

fn has_inception_derive(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("derive") {
            return false;
        }

        let syn::Meta::List(list) = &attr.meta else {
            return false;
        };
        let parser = Punctuated::<Path, syn::Token![,]>::parse_terminated;
        let Ok(derives) = parser.parse2(list.tokens.clone()) else {
            return false;
        };

        derives.iter().any(|path| {
            path.segments
                .last()
                .is_some_and(|segment| segment.ident == "Inception")
        })
    })
}

fn expand_with_properties(attr: TokenStream, input: TokenStream, properties: &[Path]) -> TokenStream {
    let args = proc_macro2::TokenStream::from(attr);
    if !args.is_empty() {
        return syn::Error::new_spanned(
            args,
            "this attribute does not accept arguments",
        )
        .into_compile_error()
        .into();
    }

    let mut item = parse_macro_input!(input as Item);
    let attrs = match attrs_mut(&mut item) {
        Ok(attrs) => attrs,
        Err(err) => return err.into_compile_error().into(),
    };

    if !has_inception_derive(attrs) {
        attrs.push(parse_quote!(#[derive(inception::Inception)]));
    }
    attrs.push(parse_quote!(#[inception(properties = [#(#properties),*])]));

    quote!(#item).into()
}

#[proc_macro_attribute]
pub fn instinct(attr: TokenStream, input: TokenStream) -> TokenStream {
    expand_with_properties(
        attr,
        input,
        &[
            parse_quote!(jungle_types::JungleFlow),
            parse_quote!(jungle_types::JungleDynFlow),
        ],
    )
}

#[proc_macro_attribute]
pub fn flow(attr: TokenStream, input: TokenStream) -> TokenStream {
    expand_with_properties(attr, input, &[parse_quote!(jungle_types::JungleFlow)])
}

#[proc_macro_attribute]
pub fn animals(attr: TokenStream, input: TokenStream) -> TokenStream {
    expand_with_properties(
        attr,
        input,
        &[
            parse_quote!(jungle_types::Ident),
            parse_quote!(jungle_types::JungleCreatures),
        ],
    )
}

#[proc_macro_attribute]
pub fn actions(attr: TokenStream, input: TokenStream) -> TokenStream {
    expand_with_properties(
        attr,
        input,
        &[
            parse_quote!(jungle_types::Ident),
            parse_quote!(jungle_types::JungleActions),
        ],
    )
}
