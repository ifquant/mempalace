# MemPalace Mine

When the user wants to mine data into the Rust MemPalace, follow these steps:

## 1. Ask what to mine

Clarify whether the source is:

- a project directory
- conversation exports
- conversations that should be auto-classified with `--extract general`

## 2. Choose the mining mode

### Project mining

    mempalace-rs mine <dir>

### Conversation mining

    mempalace-rs mine <dir> --mode convos

### General extraction

    mempalace-rs mine <dir> --mode convos --extract general

## 3. Optional flags

- `--wing <name>` to force the wing
- `--dry-run` to preview without writing
- `--progress` to print per-file progress
- `--human` for readable summaries

## 4. Suggest next steps

- `mempalace-rs search "query"`
- `mempalace-rs compress`
- `mempalace-rs wake-up`
