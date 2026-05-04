use proc_macro::TokenStream;

#[proc_macro]
pub fn noop(input: TokenStream) -> TokenStream {
    input
}
