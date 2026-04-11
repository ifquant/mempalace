# AGENTS.md

> How to build, test, and contribute to MemPalace.

## Setup

```bash
cd python
pip install -e ".[dev]"
```

## Commands

```bash
# Run tests
cd python && python -m pytest tests/ -v --ignore=tests/benchmarks

# Run tests with coverage
cd python && python -m pytest tests/ -v --ignore=tests/benchmarks --cov=mempalace --cov-report=term-missing

# Lint
cd python && ruff check .

# Format
cd python && ruff format .

# Format check (CI mode)
cd python && ruff format --check .
```

## Project structure

```
python/
├── mempalace/
│   ├── mcp_server.py      # MCP server — all read/write tools
│   ├── miner.py           # Project file miner
│   ├── convo_miner.py     # Conversation transcript miner
│   ├── searcher.py        # Semantic search
│   ├── knowledge_graph.py # Temporal entity-relationship graph (SQLite)
│   ├── palace.py          # Shared palace operations (ChromaDB access)
│   ├── config.py          # Configuration + input validation
│   ├── normalize.py       # Transcript format detection + normalization
│   ├── cli.py             # CLI dispatcher
│   ├── dialect.py         # AAAK compression dialect
│   ├── palace_graph.py    # Room traversal + cross-wing tunnels
│   ├── hooks_cli.py       # Hook system for auto-save
│   └── version.py         # Single source of truth for version
└── tests/
```

## Conventions

- **Python style**: snake_case for functions/variables, PascalCase for classes
- **Linter**: ruff with E/F/W rules
- **Formatter**: ruff format, double quotes
- **Commits**: conventional commits (`fix:`, `feat:`, `test:`, `docs:`, `ci:`)
- **Tests**: `tests/test_*.py`, fixtures in `tests/conftest.py`
- **Coverage**: 85% threshold (80% on Windows due to ChromaDB file lock cleanup)

## Architecture

```
User → CLI / MCP Server → ChromaDB (vector store) + SQLite (knowledge graph)

Palace structure:
  WING (person/project)
    └── ROOM (topic)
          └── DRAWER (verbatim text chunk)

Knowledge Graph:
  ENTITY → PREDICATE → ENTITY (with valid_from / valid_to dates)
```

## Key files for common tasks

- **Adding an MCP tool**: `python/mempalace/mcp_server.py` — add handler function + TOOLS dict entry
- **Changing search**: `python/mempalace/searcher.py`
- **Modifying mining**: `python/mempalace/miner.py` (project files) or `python/mempalace/convo_miner.py` (transcripts)
- **Input validation**: `python/mempalace/config.py` — `sanitize_name()` / `sanitize_content()`
- **Tests**: mirror source structure in `python/tests/test_<module>.py`
