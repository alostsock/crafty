use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as QuoteTokens};
use quote::quote;
use serde_derive_internals::{
    ast,
    ast::Style::{Newtype, Struct, Tuple, Unit},
    Ctxt, Derive,
};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(TsType)]
pub fn ts_type_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident.clone();

    let cx = Ctxt::new();
    let container =
        ast::Container::from_ast(&cx, &input, Derive::Deserialize).expect("invalid ast");

    if cx.check().is_err() {
        return TokenStream::from(quote! {
            compile_error!("error parsing ts_type_derive input");
        });
    }

    let ts_tokens: QuoteTokens = match container.data {
        ast::Data::Enum(variants) => {
            if let Some(tokens) = process_enum(&ident, &variants) {
                tokens
            } else {
                return TokenStream::from(quote! {
                    compile_error!("ts_type_derive does not support enums with data");
                });
            }
        }
        ast::Data::Struct(Struct, fields) => process_struct(&ident, &fields),
        ast::Data::Struct(Tuple, _)
        | ast::Data::Struct(Newtype, _)
        | ast::Data::Struct(Unit, _) => {
            return TokenStream::from(quote! {
                compile_error!("ts_type_derive does not support tuple, newtype, or unit structs");
            })
        }
    };

    let ts_string = ts_tokens.to_string();

    let tokens = quote!(
        impl TsType for #ident {
            fn ts_type() -> &'static str {
                #ts_string
            }
        }
    );

    TokenStream::from(tokens)
}

fn process_enum(ident: &syn::Ident, variants: &[ast::Variant]) -> Option<QuoteTokens> {
    let variant_idents: Vec<syn::Ident> = variants.iter().map(|v| v.ident.clone()).collect();

    if variants.iter().all(|v| v.fields.is_empty()) {
        Some(quote! {
            export type #ident = #(#variant_idents)|*;
        })
    } else {
        None
    }
}

fn process_struct(ident: &syn::Ident, fields: &[ast::Field]) -> QuoteTokens {
    let ts_fields: Vec<QuoteTokens> = fields
        .iter()
        .map(|field| {
            let name = field.attrs.name().serialize_name();
            let field_span = field
                .original
                .ident
                .clone()
                .unwrap_or_else(|| ident.clone())
                .span();
            let field_ident = syn::Ident::new(&name, field_span);
            let ty = process_type(field.ty);
            quote!(#field_ident: #ty;)
        })
        .collect();

    quote! {
        export type #ident = {
            #(#ts_fields)*
        };
    }
}

fn process_type(ty: &syn::Type) -> Option<QuoteTokens> {
    match ty {
        // Vec<T> => T[]
        syn::Type::Array(ty_array) => {
            let ty_inner = process_type(&ty_array.elem)?;
            Some(quote!(#ty_inner[]))
        }
        // [T] => T[]
        syn::Type::Slice(ty_slice) => {
            let ty_inner = process_type(&ty_slice.elem)?;
            Some(quote!(#ty_inner[]))
        }
        // (usize, String, bool) => [number, string, boolean]
        syn::Type::Tuple(ty_tuple) => {
            let ty_inner: Option<Vec<QuoteTokens>> =
                ty_tuple.elems.iter().map(process_type).collect();

            ty_inner.map(|ty_inner| quote!([#(#ty_inner),*]))
        }
        // primitives, named types
        syn::Type::Path(ty_path) => {
            let segments = &ty_path.path.segments;

            if segments.len() > 1 {
                return None;
            }

            match segments[0].ident.to_string().as_str() {
                "Option" => extract_path_argument(&segments[0]).map(|ty| quote!(#ty | undefined)),
                "Vec" => extract_path_argument(&segments[0]).map(|ty| quote!(#ty[])),
                _ => {
                    let ts_type = process_path_segment(&segments[0].ident);
                    Some(quote!(#ts_type))
                }
            }
        }
        syn::Type::BareFn(_)
        | syn::Type::Group(_)
        | syn::Type::ImplTrait(_)
        | syn::Type::Infer(_)
        | syn::Type::Macro(_)
        | syn::Type::Never(_)
        | syn::Type::Paren(_)
        | syn::Type::Ptr(_)
        | syn::Type::Reference(_)
        | syn::Type::TraitObject(_)
        | syn::Type::Verbatim(_) => None,
        _ => None,
    }
}

fn process_path_segment(ident: &Ident) -> QuoteTokens {
    match ident.to_string().as_str() {
        "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "f32" | "f64" | "usize" | "isize" => {
            quote!(number)
        }
        "u64" | "i64" | "u128" | "i128" => quote!(BigInt),
        "bool" => quote!(boolean),
        "char" | "Path" | "PathBuf" | "String" | "&'static str" => quote!(string),
        "()" => quote!(null),
        _ => quote!(#ident),
    }
}

fn extract_path_argument(path_segment: &syn::PathSegment) -> Option<QuoteTokens> {
    if let syn::PathArguments::AngleBracketed(arguments) = &path_segment.arguments {
        if arguments.args.len() == 1 {
            if let syn::GenericArgument::Type(ty_inner) = &arguments.args[0] {
                let ty = process_type(ty_inner);
                return Some(quote!(#ty));
            }
        }
    }

    None
}
