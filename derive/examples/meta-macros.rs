use std::collections::BTreeMap;

use jsonrpc_core::{futures, MetaIoHandler, Metadata, Error, Value, Result};
use jsonrpc_core::futures::future::FutureResult;
use jsonrpc_derive::rpc;

#[derive(Clone)]
struct Meta(String);
impl Metadata for Meta {}

#[rpc]
pub trait Rpc<One> {
	type Metadata;

	/// Get One type.
	#[rpc(name = "getOne")]
	fn one(&self) -> Result<One>;

	/// Adds two numbers and returns a result
	#[rpc(name = "add")]
	fn add(&self, _: u64, _: u64) -> Result<u64>;

	/// Multiplies two numbers. Second number is optional.
	#[rpc(name = "mul")]
	fn mul(&self, _: u64, _: Option<u64>) -> Result<u64>;

	/// Performs asynchronous operation
	#[rpc(name = "callAsync")]
	fn call(&self, _: u64) -> FutureResult<String, Error>;

	/// Performs asynchronous operation with meta
	#[rpc(meta, name = "callAsyncMeta", alias("callAsyncMetaAlias"))]
	fn call_meta(&self, _: Self::Metadata, _: BTreeMap<String, Value>) -> FutureResult<String, Error>;
}

struct RpcImpl;
impl Rpc<u64> for RpcImpl {
	type Metadata = Meta;

	fn one(&self) -> Result<u64> { Ok(100) }

	fn add(&self, a: u64, b: u64) -> Result<u64> {
		Ok(a + b)
	}

	fn mul(&self, a: u64, b: Option<u64>) -> Result<u64> {
		Ok(a * b.unwrap_or(1))
	}

	fn call(&self, x: u64) -> FutureResult<String, Error> {
		futures::finished(format!("OK: {}", x))
	}

	fn call_meta(&self, meta: Self::Metadata, map: BTreeMap<String, Value>) -> FutureResult<String, Error> {
		futures::finished(format!("From: {}, got: {:?}", meta.0, map))
	}
}


fn main() {
	let mut io = MetaIoHandler::default();
	let rpc = RpcImpl;

	io.extend_with(rpc.to_delegate());

	let server = jsonrpc_tcp_server::ServerBuilder
		::with_meta_extractor(io, |context: &jsonrpc_tcp_server::RequestContext| {
			Meta(format!("{}", context.peer_addr))
		})
		.start(&"0.0.0.0:3030".parse().unwrap())
		.expect("Server must start with no issues");

	server.wait()
}
