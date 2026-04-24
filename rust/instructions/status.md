# MemPalace Status

Display the current state of the Rust palace.

## Step 1: Gather status

Prefer MCP:

- `mempalace_status`
- `mempalace_kg_stats`
- `mempalace_graph_stats`

CLI fallback:

    mempalace-rs status

## Step 2: Show concise counts

Summarize:

- total drawers
- wings
- rooms
- schema version

## Step 3: Suggest one next step

- Empty palace: `mempalace-rs mine <dir>`
- Healthy palace: `mempalace-rs search "query"`
- If richer context is needed: `mempalace-rs wake-up`
