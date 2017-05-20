#![feature(proc_macro)]

#[macro_use] extern crate lazy_static;
extern crate proc_macro;
extern crate syn;
extern crate regex;
#[macro_use] extern crate quote;

use std::str::FromStr;

use proc_macro::TokenStream;

mod _impl;

#[proc_macro]
pub fn gen_traits_and_impls(input: TokenStream) -> TokenStream {
    // Construct a string representation of the type definition
    let source = input.to_string();

    // Parse the string representation into a syntax tree
    let ast = syn::parse_expr(&source).unwrap();

    // Build the output
    match _impl::gen_traits_and_impls(ast) {
        Ok(tokens) => TokenStream::from_str(tokens.as_str()).unwrap(),
        Err(error) => panic!(error),
    }
}