extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Ident};

#[proc_macro_derive(EnumIndexing)]
pub fn enum_indexing_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let enum_variants = if let Data::Enum(enum_data) = input.data {
        enum_data.variants
    } else {
        return TokenStream::from(
            quote! { compile_error!("#[derive(EnumIndexing)] is only allowed on enums"); },
        );
    };

    let variant_count: usize = enum_variants.len();
    let variant_index: Vec<usize> = (0..variant_count).collect();
    let variant_name: Vec<Ident> = enum_variants.iter().map(|v| v.ident.clone()).collect();

    let tokens = quote! {
        impl EnumIndexing for #name {
            fn index(&self) -> usize {
                match self {
                    #( #name::#variant_name => #variant_index, )*
                }
            }

            fn from_index(index: usize) -> Option<Self> {
                match index {
                    #( #variant_index => Some(#name::#variant_name), )*
                    _ => None,
                }
            }

            fn count() -> usize {
                #variant_count
            }
        }
    };

    TokenStream::from(tokens)
}
