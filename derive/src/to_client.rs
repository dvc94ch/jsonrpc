use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;
use crate::rpc_attr::AttributeKind;
use crate::to_delegate::MethodRegistration;

pub fn generate_client_module(
	  methods: &[MethodRegistration],
	  rpc_trait: &syn::ItemTrait,
) -> Result<TokenStream> {
    let mut method_decl: Vec<syn::TraitItem> = vec![];
    let mut method_impl: Vec<syn::ImplItem> = vec![];
    let mut method_resp: Vec<syn::Item> = vec![];
    for method in methods {
        match method {
            MethodRegistration::Standard { method, .. } => {
                let rpc_name = method.name();
                let name = &method.trait_item.sig.ident;
                let returns = match &method.attr.kind {
                    AttributeKind::Rpc { returns: Some(returns), .. } => returns,
                    _ => continue,
                };
                let returns: syn::Type = syn::parse_str(returns)?;
                let new_ty: syn::Type = syn::parse_quote! {
                    future::AndThen<
                        FutureResult<String, Error>,
                    FutureResult<#returns, Error>,
                    fn(String) -> FutureResult<#returns, Error>>
                };
                let args = &method.trait_item.sig.decl.inputs;
                let arg_names: Vec<&syn::Ident> = args
                    .iter().filter_map(|arg| {
                        match arg {
                            syn::FnArg::Captured(syn::ArgCaptured {
                                pat: syn::Pat::Ident(syn::PatIdent { ident, .. }),
                                ..
                            }) => Some(ident),
                            _ => None,
                        }
                    }).collect();
                method_decl.push(syn::parse_quote! {
                    fn #name(#args) -> #new_ty;
                });
                method_impl.push(syn::parse_quote! {
                    fn #name(#args) -> #new_ty {
                        let args_tuple = (#(#arg_names,)*);
                        let args = serde_json::to_value(args_tuple)
                            .expect("should never fail");
                        let params = serde_json::from_value(args)
                            .expect("should never fail");
                        let request = Request::Single(Call::MethodCall(MethodCall {
                            jsonrpc: Some(Version::V2),
                            method: #rpc_name.to_owned(),
                            params,
                            id: Id::Num(next_id()),
                        }));
                        let request_str = serde_json::to_string(&request)
                            .expect("should never fail");
                        self.call_method(request_str)
                            .and_then(rpc_impl_response::#name)
                    }
                });
                method_resp.push(syn::parse_quote! {
                    pub fn #name(response_str: String) -> FutureResult<#returns, Error> {
                        let response = serde_json::from_str::<Response>(&response_str)
                            .map_err(|_| Error::new(ErrorCode::ParseError));
                        let response = match response {
                            Ok(response) => response,
                            Err(error) => return future::err(error),
                        };
                        let value: Result<Value> = match response {
                            Response::Single(output) => output.into(),
                            _ => return future::err(Error::parse_error()),
                        };
                        let value = match value {
                            Ok(value) => value,
                            Err(error) => return future::err(error),
                        };
                        let result = serde_json::from_value::<#returns>(value)
                            .map_err(|_| Error::new(ErrorCode::ParseError));
                        match result {
                            Ok(result) => future::ok(result),
                            Err(error) => future::err(error),
                        }
                    }
                })
            }
            _ => {}
        }
    }

    let mut rpc_client_trait = rpc_trait.to_owned();
    rpc_client_trait.items = method_decl;

    let trait_name = &rpc_client_trait.ident;
    let rpc_client_impl: syn::ItemImpl = syn::parse_quote! {
        impl<T: jsonrpc_core::client::RpcClient> #trait_name for T {
            #(#method_impl)*
        }
    };

    let response_mod: syn::ItemMod = syn::parse_quote! {
        mod rpc_impl_response {
            use super::*;
            #(#method_resp)*
        }
    };

    Ok(quote! {
        pub mod client {
            use super::*;
            use jsonrpc_core::{
                Call, Error, ErrorCode, Id, MethodCall, Params, Request,
                Response, Version,
            };
            use jsonrpc_core::futures::future::Future;
            use serde_json::Value;
            use std::sync::atomic::{AtomicU64, Ordering};

            static CURR_ID: AtomicU64 = AtomicU64::new(0);

            #[inline]
            fn next_id() -> u64 {
                CURR_ID.fetch_add(1, Ordering::SeqCst)
            }

            #rpc_client_trait

            #rpc_client_impl

            #response_mod
        }
    })
}
