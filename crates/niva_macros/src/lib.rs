use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, parse_str,
    punctuated::Punctuated,
    token::{Comma, Semi},
    FnArg, ItemFn, LitInt, Pat, Stmt, Type,
};

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let path_segment = &segment.ident;
            return path_segment == "Option";
        }
    }
    false
}

fn niva_api_args(api_inputs: Punctuated<FnArg, Comma>) -> Option<Stmt> {
    let len = api_inputs.len();

    if len == 0 {
        None
    } else if len == 1 {
        let arg = api_inputs.first().unwrap();
        match arg {
            FnArg::Typed(typed) => {
                let ty = &typed.ty;
                let pat = &typed.pat;
                Some(parse_quote! {
                    let #pat = request.args().single::<#ty>()?;
                })
            }
            _ => None,
        }
    } else {
        let mut names: Punctuated<Box<Pat>, Comma> = Punctuated::new();
        let mut types: Punctuated<Box<Type>, Comma> = Punctuated::new();
        let len = parse_str::<LitInt>(&len.to_string()).unwrap();

        for arg in api_inputs {
            match arg {
                FnArg::Typed(typed) => {
                    let ty = &typed.ty;
                    let pat = &typed.pat;
                    names.push(pat.clone());
                    types.push(ty.clone());
                }
                _ => {}
            }
        }

        let has_option = types.iter().any(|ty| is_option_type(ty));

        if has_option {
            Some(parse_quote! {
                let (#names) = request.args().optional::<(#types)>(#len)?;
            })
        } else {
            Some(parse_quote! {
                let (#names) = request.args().get::<(#types)>()?;
            })
        }
    }
}

#[proc_macro_attribute]
pub fn niva_api(_: TokenStream, raw_item: TokenStream) -> TokenStream {
    let define = parse_macro_input!(raw_item as ItemFn);

    let name = define.sig.ident;
    let inputs = define.sig.inputs;
    let output = define.sig.output;

    let mut stmts = Punctuated::<syn::Stmt, Semi>::new();
    define.block.stmts.into_iter().for_each(|stmt| {
        stmts.push(stmt);
    });

    let app_ty = quote! {std::sync::Arc<crate::app::NivaApp>};
    let window_ty = quote! {std::sync::Arc<crate::app::window_manager::window::NivaWindow>};
    let request_ty = quote! {crate::app::api_manager::ApiRequest};

    let args = niva_api_args(inputs);

    TokenStream::from(quote! {
        fn #name(app: #app_ty, window: #window_ty, request: #request_ty) #output {
            #args
            #stmts
        }
    })
}


#[proc_macro_attribute]
pub fn niva_event_api(_: TokenStream, raw_item: TokenStream) -> TokenStream {
    let define = parse_macro_input!(raw_item as ItemFn);

    let name = define.sig.ident;
    let inputs = define.sig.inputs;
    let output = define.sig.output;

    let mut stmts = Punctuated::<syn::Stmt, Semi>::new();
    define.block.stmts.into_iter().for_each(|stmt| {
        stmts.push(stmt);
    });

    let app_ty = quote! {std::sync::Arc<crate::app::NivaApp>};
    let window_ty = quote! {std::sync::Arc<crate::app::window_manager::window::NivaWindow>};
    let request_ty = quote! {crate::app::api_manager::ApiRequest};
    let target_ty = quote! {&crate::app::NivaWindowTarget};
    let control_flow_ty = quote! {&mut tao::event_loop::ControlFlow};

    let args = niva_api_args(inputs);

    TokenStream::from(quote! {
        fn #name(app: #app_ty, window: #window_ty, request: #request_ty, target: #target_ty, control_flow: #control_flow_ty) #output {
            #args
            #stmts
        }
    })
}