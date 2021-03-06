// Copyright (c) 2017-present PyO3 Project and Contributors

use syn;
use quote::Tokens;

use py_method;


pub fn build_py_methods(ast: &mut syn::Item) -> Tokens {
    match ast.node {
        syn::ItemKind::Impl(_, _, _, ref path, ref ty, ref mut impl_items) => {
            if let &Some(_) = path {
                panic!("#[methods] can not be used only with trait impl block");
            } else {
                impl_methods(ty, impl_items)
            }
        },
        _ => panic!("#[methods] can only be used with Impl blocks"),
    }
}

fn impl_methods(ty: &Box<syn::Ty>, impls: &mut Vec<syn::ImplItem>) -> Tokens {

    // get method names in impl block
    let mut methods = Vec::new();
    for iimpl in impls.iter_mut() {
        match iimpl.node {
            syn::ImplItemKind::Method(ref mut sig, ref mut block) => {
                methods.push(py_method::gen_py_method(
                    ty, &iimpl.ident, sig, block, &mut iimpl.attrs));
            },
            _ => (),
        }
    }

    let tokens = quote! {
        impl pyo3::class::methods::PyMethodsProtocolImpl for #ty {
            fn py_methods() -> &'static [pyo3::class::PyMethodDefType] {
                static METHODS: &'static [pyo3::class::PyMethodDefType] = &[
                    #(#methods),*
                ];
                METHODS
            }
        }
    };

    let dummy_const = syn::Ident::new("_IMPL_PYO3_METHODS");
    quote! {
        #[feature(specialization)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            extern crate pyo3;
            use pyo3::ffi;

            #tokens
        };
    }
}
