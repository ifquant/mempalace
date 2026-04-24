use serde_json::{Value, json};

pub use crate::mcp_schema_support::{
    PALACE_PROTOCOL, SUPPORTED_PROTOCOL_VERSIONS, coerce_argument_types, negotiate_protocol,
    no_palace, required_str, requires_existing_palace, string_list_arg, truncate_duplicate_content,
};

pub fn tools() -> Vec<Value> {
    let mut tools = Vec::new();
    tools.extend(crate::mcp_schema_catalog_read::tools());
    tools.extend(crate::mcp_schema_catalog_write::tools());
    tools.extend(crate::mcp_schema_catalog_project::tools());
    tools.extend(crate::mcp_schema_catalog_registry::tools());
    tools
}

pub fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}
