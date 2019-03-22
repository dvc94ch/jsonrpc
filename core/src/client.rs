//! Defines the `RpcClient` trait.
use crate::{Error, IoHandler};
use crate::futures::future::{self, FutureResult};

/// The RpcClient trait. Use the #[rpc] macro derives the Request and Response
/// generation.
pub trait RpcClient: Send + Sync + 'static {
    /// Calls an rpc method.
    fn call_method(&self, request: String) -> FutureResult<String, Error>;
}

impl RpcClient for IoHandler {
    fn call_method(&self, request: String) -> FutureResult<String, Error> {
        future::ok(self.handle_request_sync(&request).unwrap())
    }
}
