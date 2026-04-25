//! MCP schema catalog for the Rust rewrite.
//!
//! This module assembles tool definitions from the read, write, project, and
//! registry catalogs so auditors can inspect the full public MCP surface in one
//! place.

use serde_json::{Value, json};

pub use crate::mcp_schema_support::{
    PALACE_PROTOCOL, SUPPORTED_PROTOCOL_VERSIONS, coerce_argument_types, negotiate_protocol,
    no_palace, required_str, requires_existing_palace, string_list_arg, truncate_duplicate_content,
};

/// Return the complete MCP tool list exposed by the Rust runtime.
pub fn tools() -> Vec<Value> {
    let mut tools = Vec::new();
    tools.extend(crate::mcp_schema_catalog_read::tools());
    tools.extend(crate::mcp_schema_catalog_write::tools());
    tools.extend(crate::mcp_schema_catalog_project::tools());
    tools.extend(crate::mcp_schema_catalog_registry::tools());
    tools
}

/// Build one MCP tool descriptor in the schema shape expected by clients.
pub fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}
