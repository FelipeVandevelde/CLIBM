//! Pluggable transports for CLIBM MCP.

#[cfg(feature = "transport-stdio")]
pub mod stdio;

#[cfg(feature = "transport-http")]
pub mod http;

use crate::adapter::McpAdapter;
use anyhow::Result;
use std::sync::Arc;

#[allow(dead_code)]
pub trait Transport {
    fn run(&self, adapter: Arc<McpAdapter>) -> Result<()>;
}
