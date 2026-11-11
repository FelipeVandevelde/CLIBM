//! Library crate for CLIBM MCP protocol handling, adapters, and transports.

pub mod adapter;
pub mod backend;
pub mod config;
pub mod error;
pub mod protocol;
pub mod transport;

pub use adapter::McpAdapter;
pub use config::{McpConfig, TransportMode};
pub use error::JsonRpcError;
pub use protocol::{JsonRpcRequest, JsonRpcResponse};
