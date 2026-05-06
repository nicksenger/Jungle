use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Path};

mod inception_derive;

#[proc_macro]
pub fn noop(input: TokenStream) -> TokenStream {
    input
}

fn derive_with_properties(input: TokenStream, properties: &[Path]) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    input
        .attrs
        .push(parse_quote!(#[inception(properties = [#(#properties),*])]));
    inception_derive::State::gen(quote!(#input).into())
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
