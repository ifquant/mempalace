use serde_json::{Value, json};

use crate::mcp_schema::tool;

pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "mempalace_onboarding",
            "Bootstrap a project-local world model and registry files.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory to bootstrap"},
                    "mode": {"type":"string","description":"Onboarding mode: work, personal, or combo"},
                    "people": {"type":"array","items":{"type":"string"},"description":"People in name,relationship,context format"},
                    "projects": {"type":"array","items":{"type":"string"},"description":"Project names to seed"},
                    "aliases": {"type":"array","items":{"type":"string"},"description":"Alias mappings in alias=canonical format"},
                    "wings": {"type":"array","items":{"type":"string"},"description":"Wing names to seed"},
                    "scan": {"type":"boolean","description":"Scan local files for additional names"},
                    "auto_accept_detected": {"type":"boolean","description":"Auto-accept detected names during scan"}
                },
                "required": ["project_dir"]
            }),
        ),
        tool(
            "mempalace_normalize",
            "Normalize one chat export or transcript into MemPalace conversation format.",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type":"string","description":"Chat export or transcript file to normalize"}
                },
                "required": ["file_path"]
            }),
        ),
        tool(
            "mempalace_split",
            "Split transcript mega-files into per-session text files.",
            json!({
                "type": "object",
                "properties": {
                    "source_dir": {"type":"string","description":"Directory containing transcript files"},
                    "output_dir": {"type":"string","description":"Optional output directory for split files"},
                    "min_sessions": {"type":"integer","description":"Only split files with at least this many sessions (default 2)"},
                    "dry_run": {"type":"boolean","description":"Preview without writing files"}
                },
                "required": ["source_dir"]
            }),
        ),
        tool(
            "mempalace_instructions",
            "Return one built-in MemPalace instruction document.",
            json!({
                "type": "object",
                "properties": {
                    "name": {"type":"string","description":"Instruction set name: help, init, mine, search, or status"}
                },
                "required": ["name"]
            }),
        ),
        tool(
            "mempalace_hook_run",
            "Run one MemPalace hook using explicit MCP arguments instead of stdin.",
            json!({
                "type": "object",
                "properties": {
                    "hook": {"type":"string","description":"Hook name: session-start, stop, or precompact"},
                    "harness": {"type":"string","description":"Harness type: claude-code or codex"},
                    "session_id": {"type":"string","description":"Session identifier"},
                    "stop_hook_active": {"type":"boolean","description":"Whether the stop hook is already active"},
                    "transcript_path": {"type":"string","description":"Optional transcript JSONL path for stop counting"}
                },
                "required": ["hook", "harness"]
            }),
        ),
    ]
}
