# MemPalace Init

Guide the user through a complete Rust MemPalace setup.

## Step 1: Check if the palace already exists

Run:

    mempalace-rs status

If the palace already exists, report that and skip to Step 4.

## Step 2: Ask for the project directory

Ask the user which project directory they want to initialize with MemPalace.

## Step 3: Initialize the palace

Run:

    mempalace-rs init <dir>

If this fails, report the error and stop.

## Step 4: Mine initial content

Suggest one of:

    mempalace-rs mine <dir>
    mempalace-rs mine <dir> --mode convos

## Step 5: Show next steps

- `mempalace-rs search "query"`
- `mempalace-rs wake-up`
- `mempalace-rs status`
