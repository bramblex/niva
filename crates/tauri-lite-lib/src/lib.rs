extern crate proc_macro;
use darling;
use proc_macro::{TokenStream};
use quote::quote;
use syn::{self, parse_macro_input, AttributeArgs, ItemFn, PatType, punctuated::Punctuated, token::Comma};

/*
    #[tl_api]
    fn test_api(context: context, arg: Arg) -> Result<Data, Error> {
    }
*/

fn transform_params(params: Punctuated<syn::FnArg, syn::token::Comma>) -> Punctuated<syn::Ident, Comma> {
    // 1. Filter the params, so that only typed arguments remain
    // 2. Extract the ident (in case the pattern type is ident)
    let idents = params.iter().filter_map(|param| {
        if let syn::FnArg::Typed(pat_type) = param {
            if let syn::Pat::Ident(pat_ident) = *pat_type.pat.clone() {
                return Some(pat_ident.ident);
            }
        }
        None
    });

    // Add all idents to a Punctuated => param1, param2, ...
    let mut punctuated: Punctuated<syn::Ident, Comma> = Punctuated::new();
    idents.for_each(|ident| punctuated.push(ident));

    return punctuated;
}

fn transform_types(params: Punctuated<syn::FnArg, syn::token::Comma>) -> Punctuated<Box<syn::Type>, Comma> {
    // 1. Filter the params, so that only typed arguments remain
    // 2. Extract the ident (in case the pattern type is ident)
    let types = params.iter().filter_map(|param| {
        if let syn::FnArg::Typed(pat_type) = param {
            return Some(pat_type.ty.clone());
        }
        None
    });

    // Add all idents to a Punctuated => param1, param2, ...
    let mut punctuated: Punctuated<Box<syn::Type>, Comma> = Punctuated::new();
    types.for_each(|ty| punctuated.push(ty));

    return punctuated;
}

#[proc_macro_attribute]
pub fn tl_api(raw_attr: TokenStream, raw_item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(raw_attr as AttributeArgs);
    let item = syn::parse_macro_input!(raw_item as ItemFn);

    let id = item.sig.ident;

    let inputs = item.sig.inputs;
    let output = item.sig.output;
    let body = item.block;

    let args = transform_params(inputs.clone());
    let argTypes = transform_types(inputs.clone());
    let args_len = args.len();

    let expanded = quote! {
        fn #id (req: ApiRequest) -> ApiResponse {
            let func = |#inputs| #output #body;
            let mut req_args = req.args;

            for i in req_args.len()..#args_len {
                req_args.push(serde_json::Value::Null);
            }

            let (#args) = serde_json::from_value::<(#argTypes)>(serde_json::Value::Array(req_args)).unwrap();
            let result = func(#args).unwrap();

            ApiResponse {
                callback_id: req.callback_id,
                code: 0,
                msg: "ok".to_string(),
                data: serde_json::to_value(result).unwrap(),
            }
        }
    };

    TokenStream::from(expanded)
}
