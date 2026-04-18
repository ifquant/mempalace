# MemPalace Search

When the user wants to search their Rust MemPalace memories:

## 1. Parse the query

Identify:

- semantic query text
- optional `wing`
- optional `room`

## 2. Prefer MCP when available

Use:

- `mempalace_search`
- `mempalace_list_wings`
- `mempalace_list_rooms`
- `mempalace_get_taxonomy`
- `mempalace_traverse`
- `mempalace_find_tunnels`

## 3. CLI fallback

    mempalace-rs search "query" [--wing X] [--room Y]

## 4. Present results

- include wing / room / source
- include similarity if present
- keep results grouped or labeled clearly

## 5. Offer follow-up

- narrow by wing or room
- run `mempalace-rs wake-up`
- explore taxonomy or graph tools
