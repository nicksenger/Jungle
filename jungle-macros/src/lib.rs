use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Path};

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

    let item = parse_macro_input!(input as syn::Item);
    quote! {
        #[inception::inception_derive(properties = [#(#properties),*])]
        #item
    }
    .into()
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
