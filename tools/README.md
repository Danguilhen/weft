# Tools

| Tool | Purpose |
|---|---|
| `check_docs.py` | Verify relative Markdown links and required design files. Offline and dependency-free. |
| `graphify.sh` | Refresh the optional Graphify code knowledge graph, with the project excludes. |

Run from the repository root:

```bash
python3 tools/check_docs.py
./tools/graphify.sh
```

`graphify` is optional. The wrapper prints installation instructions when the CLI
is missing and does nothing until Rust sources exist. The graph is written to
`graphify-out/` and is gitignored.

There is no Rust workspace yet. Do not add a build tool that is not documented in
`AGENTS.md`.
