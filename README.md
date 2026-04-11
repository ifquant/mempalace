# MemPalace

This repository is being reorganized to host multiple implementations.

- Python implementation: [python/README.md](python/README.md)
- Python package root: [python/pyproject.toml](python/pyproject.toml)
- Shared repository assets and integrations remain at the repo root: `hooks/`, `docs/`, `assets/`, `.codex-plugin/`, `.claude-plugin/`, `integrations/`

Current development commands for the Python implementation:

```bash
cd python
pip install -e ".[dev]"
python -m pytest tests/ -v --ignore=tests/benchmarks
ruff check .
```
