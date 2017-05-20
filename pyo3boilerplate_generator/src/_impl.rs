use syn;
use quote::{Tokens, ToTokens};

type Result<T> = ::std::result::Result<T, &'static str>;

#[inline]
fn extract_ident_from_path(mut path: syn::Path) -> Result<syn::Ident> {
    let res = path.segments.pop().unwrap().ident;
    if path.global || !path.segments.is_empty() {
        Err("expected an ident")
    } else {
        Ok(res)
    }
}

enum StructOrMethod {
    Struct(syn::Ident),
    Method(syn::Ident),
}

fn extract_struct_or_method_and_fields_helper(
    mut path: syn::Path,
    fields: Vec<syn::FieldValue>,
    ) -> Result<(StructOrMethod, Vec<syn::FieldValue>)>
{
    if path.global {
        Err("unexpected global path")
    } else {
        let ident = path.segments.pop().unwrap().ident;
        let what = path.segments.pop().map(|x| x.ident).unwrap_or(syn::Ident::new("Method"));
        let res = match what.as_ref() {
            "Method" => Ok((StructOrMethod::Method(ident), fields)),
            "Struct" => Ok((StructOrMethod::Struct(ident), fields)),
            _ => Err("unexpected path"),
        };

        if path.segments.is_empty() {
            res
        } else {
            Err("unexpected path")
        }
    }
}

fn extract_struct_or_method_and_fields(expr: syn::Expr) -> Result<(StructOrMethod, Vec<syn::FieldValue>)> {
    match expr.node {
        syn::ExprKind::Struct(path, fields, ..) =>
            extract_struct_or_method_and_fields_helper(path, fields),
        syn::ExprKind::Path(_, path) =>
            extract_struct_or_method_and_fields_helper(path, Vec::new()),
        _ => Err("expected a struct expression"),
    }
}

fn extract_ident_from_expr(expr: syn::Expr) -> Result<syn::Ident> {
    if let syn::ExprKind::Path(_, path) = expr.node {
        Ok(extract_ident_from_path(path)?)
    } else {
        Err("expected an identifier")
    }
}

struct Method {
    field: syn::Ident,
    name: syn::Ident,
    arguments: Vec<syn::Ident>,
    success: syn::Ident,
}

struct Struct {
    field: Option<syn::Ident>,
    name: syn::Ident,
    methods: Vec<Method>,
    substructs: Vec<Struct>,
}

fn process_fields(parent: &mut Struct, fields: Vec<syn::FieldValue>) -> Result<()> {
    for syn::FieldValue { ident: field, expr: value, .. } in fields.into_iter() {
        let value = match value.node {
            syn::ExprKind::Lit(syn::Lit::Int(0, _)) => {
                // TODO: implement null field
                unimplemented!()
            },
            _ => extract_struct_or_method_and_fields(value)?,
        };

        match value {
            (StructOrMethod::Struct(name), subfields) => {
                let mut substruct = Struct {
                    field: Some(field),
                    name,
                    methods: Vec::new(),
                    substructs: Vec::new(),
                };

                process_fields(&mut substruct, subfields)?;

                parent.substructs.push(substruct);
            },
            (StructOrMethod::Method(name), subfields) => {
                parent.methods.push(create_method(name, field, subfields)?);
            },
        }
    }

    Ok(())
}

fn create_method(name: syn::Ident, field: syn::Ident, subfields: Vec<syn::FieldValue>) -> Result<Method> {
    let mut arguments = Vec::new();
    let mut success = syn::Ident::new("Success");
    for syn::FieldValue { ident: key, expr: value, .. } in subfields.into_iter() {
        match key.as_ref() {
            "arguments" => {
                match value.node {
                    syn::ExprKind::Path(_, path) =>
                        arguments = vec![extract_ident_from_path(path)?],
                    syn::ExprKind::Array(argument_list) => {
                        arguments.reserve(argument_list.len());
                        for argument in argument_list {
                            arguments.push(extract_ident_from_expr(argument)?);
                        }
                    },
                    _ => return Err("unexpected arguments"),
                };
            },
            "success" => success = extract_ident_from_expr(value)?,
            _ => return Err("unexpected field"),
        };
    }

    Ok(Method {
        field,
        name,
        arguments,
        success,
    })
}

fn name_to_trait_name(name: &syn::Ident, parents: &[&Struct]) -> syn::Ident {
    let name = name.as_ref();
    let parents_len: usize = parents.iter().map(|x| x.name.as_ref().len()).sum();
    let mut res = String::with_capacity("Py".len() + parents_len + name.len() + "Protocol".len());
    res += "Py";
    for parent in parents {
        res += parent.name.as_ref();
    }
    res += name;
    res += "Protocol";
    syn::Ident::new(res)
}

fn get_trait_name_and_parent_trait_name(name: &syn::Ident, parents: &[&Struct]) -> (syn::Ident, Option<syn::Ident>) {
    let trait_name = name_to_trait_name(name, parents);
    if let Some((parent, parents)) = parents.split_last() {
        (trait_name, Some(name_to_trait_name(&parent.name, parents)))
    } else {
        (trait_name, None)
    }
}

fn output_struct_traits(root: &Struct, parents: &Vec<&Struct>) -> Tokens {
    let (trait_name, parent_trait_name) = get_trait_name_and_parent_trait_name(&root.name, &**parents);

    let mut parents = parents.clone();
    parents.push(root);

    let method_traits = root.methods.iter().map(|method| output_method_trait(method, &*parents));
    let child_struct_traits = root.substructs.iter().map(|substruct| output_struct_traits(substruct, &parents));
    let method_signatures = root.methods.iter().map(|method| output_method_signature(method, &*parents));

    let child_traits = quote! {
        #(
            #method_traits
        )*
        #(
            #child_struct_traits
        )*
    };

    if let Some(parent_trait_name) = parent_trait_name {
        quote! {
            #child_traits
            pub trait #trait_name: #parent_trait_name {
                #(
                    #method_signatures
                )*
            }
        }
    } else {
        quote! {
            #child_traits
            pub trait #trait_name {
                #(
                    #method_signatures
                )*
            }
        }
    }
}

fn output_method_trait(method: &Method, parents: &[&Struct]) -> Tokens {
    let Method { ref name, ref arguments, ref success, ..} = *method;
    let (trait_name, parent_trait_name) = get_trait_name_and_parent_trait_name(name, parents);

    // TODO: deal with non-python arguments and return
    quote! {
        pub trait #name: #parent_trait_name {
            #(type #arguments: for<'a> FromPyObject<'a>;)*
            type #success: ToPyObject;
        }
    }
}

fn output_method_signature(method: &Method, parents: &[&Struct]) -> Tokens {
    let Method { ref name, ref arguments, ref success, ..} = *method;

    let trait_name = name_to_trait_name(name, parents);

    let method_name = {
        let name = name.as_ref().to_lowercase();
        let mut method_name = String::with_capacity("__".len() + name.len() + "__".len());
        method_name += "__";
        method_name += &name;
        method_name += "__";
        syn::Ident::new(method_name)
    };

    // TODO: deal with non-python arguments and return
    let arguments = arguments.iter().map(|argument| {
        use regex::Regex;
        lazy_static! {
            static ref CAMEL_CASE_MATCHER: Regex = Regex::new(r"([A-Z][a-z]*)").unwrap();
        }
        let identifier = syn::Ident::new(CAMEL_CASE_MATCHER
            .captures_iter(argument.as_ref())
            .enumerate()
            .fold(String::new(), |mut partial, (index, text)| {
                if index > 0 {
                    partial += "_";
                }
                partial += &text[1].to_lowercase();
                partial
            }));

        quote! { #identifier: Self::#argument }
    });

    quote! {
        fn #method_name(&self, py: Python, #(#arguments,)*) -> PyResult<Self::#success>
            where Self::#trait_name { unimplemented!() }
    }
}

pub fn gen_traits_and_impls(expr: syn::Expr) -> Result<Tokens> {
    if let (StructOrMethod::Struct(name), fields) = extract_struct_or_method_and_fields(expr)? {
        let mut root = Struct {
            field: None,
            name,
            methods: Vec::new(),
            substructs: Vec::new(),
        };

        process_fields(&mut root, fields)?;

        Ok(output_struct_traits(&root, &Vec::new()))
    } else {
        Err("expected a struct at the top")
    }
}