mod page;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Page, attributes(page, table, action, column))]
pub fn derive_page(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let ident = input.ident.to_string();
    let stream = page::generate(input)
        .unwrap_or_else(|(text, span)| syn::Error::new(span, text).into_compile_error());
    std::fs::write(format!("target/page_{}.rs", ident), stream.to_string()).unwrap();
    stream.into()
}
