# MemPalace

AI memory system. Store everything, find anything. Local, free, no API key.

---

## Slash Commands

| Command              | Description                    |
|----------------------|--------------------------------|
| /mempalace:init      | Install and set up MemPalace   |
| /mempalace:search    | Search your memories           |
| /mempalace:mine      | Mine projects and conversations|
| /mempalace:status    | Palace overview and stats      |
| /mempalace:help      | This help message              |

---

## MCP Tools

- mempalace_status
- mempalace_list_wings
- mempalace_list_rooms
- mempalace_get_taxonomy
- mempalace_search
- mempalace_check_duplicate
- mempalace_get_aaak_spec
- mempalace_add_drawer
- mempalace_delete_drawer
- mempalace_kg_query
- mempalace_kg_add
- mempalace_kg_invalidate
- mempalace_kg_timeline
- mempalace_kg_stats
- mempalace_traverse
- mempalace_find_tunnels
- mempalace_graph_stats
- mempalace_diary_write
- mempalace_diary_read

---

## CLI Commands

    mempalace-rs init <dir>                  Initialize a new palace
    mempalace-rs mine <dir>                  Mine a project (default mode)
    mempalace-rs mine <dir> --mode convos    Mine conversation exports
    mempalace-rs search "query"              Search your memories
    mempalace-rs compress                    Compress drawers into AAAK summaries
    mempalace-rs wake-up                     Load L0 + L1 wake-up context
    mempalace-rs status                      Show palace status
    mempalace-rs repair                      Run diagnostics
    mempalace-rs migrate                     Upgrade palace schema
    mempalace-rs doctor                      Inspect embedding runtime
    mempalace-rs prepare-embedding           Warm the embedding runtime
    mempalace-rs hook run                    Run hook logic (stdin JSON -> stdout JSON)
    mempalace-rs instructions <name>         Output skill instructions

---

## Auto-Save Hooks

- Stop hook -- counts human messages in the session transcript and blocks every 15 messages with a save instruction.
- Precompact hook -- always blocks with a save-everything instruction before context compaction.
- Session-start hook -- initializes per-session tracking state.

Rust stores hook state under `<palace>/hook_state/` to keep the hook workflow local to the active palace.
