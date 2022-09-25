pub use ts_type_derive::TsType;
pub use wasm_bindgen::prelude::wasm_bindgen;

/// Basically a simplified version of
/// https://timryan.org/2019/01/22/exporting-serde-types-to-typescript.html
/// and https://github.com/Aleph-Alpha/ts-rs
pub trait TsType {
    fn ts_type() -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    macro_rules! assert_ast_eq {
        ($enum_or_struct_name:ident, $quote_expression:expr) => {
            assert_eq!(
                $enum_or_struct_name::ts_type()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
                $quote_expression
                    .to_string()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        };
    }

    #[allow(dead_code)]
    #[derive(TsType)]
    enum Letter {
        A,
        B,
        C,
    }

    #[test]
    fn simple_enums_work() {
        assert_ast_eq!(
            Letter,
            quote! {
                export type Letter = "A" | "B" | "C";
            }
        );
    }

    #[allow(dead_code)]
    #[derive(TsType)]
    struct Foo1 {
        first: u32,
        second: i64,
        third: bool,
    }

    #[test]
    fn simple_structs_work() {
        assert_ast_eq!(
            Foo1,
            quote! {
                export type Foo1 = {
                    first: number;
                    second: BigInt;
                    third: boolean;
                };
            }
        );
    }

    #[allow(dead_code)]
    #[derive(TsType)]
    struct Foo2 {
        first: Letter,
        second: Vec<Letter>,
        third: Option<Letter>,
        fourth: [Letter; 2],
    }

    #[test]
    fn less_simple_structs_work() {
        assert_ast_eq!(
            Foo2,
            quote! {
                export type Foo2 = {
                    first: Letter;
                    second: Letter[];
                    third: Letter | undefined;
                    fourth: Letter[];
                };
            }
        );
    }
}
