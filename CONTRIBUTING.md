# Contributing

Thanks for working on AikenFlow. Keep the product boundary intact:

```text
AikenFlow Rust core = protocol semantics, validation, generation, assurance
VS Code extension = orchestration, diagnostics, webview hosting
React webview = graph, assurance, and artefact rendering
```

Do not move protocol semantics into the VS Code extension or webview.

## Local Gates

Run these before opening a pull request:

```bash
cargo fmt --all -- --check
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo audit
```

For the VS Code extension:

```bash
cd extensions/vscode-aikenflow
npm ci
npm audit --audit-level=moderate
npm run check:full
```

For release candidates, install Aiken and run:

```bash
bash scripts/verify-examples.sh
```

## Generated Outputs

Generated artefacts are deterministic and protected by `.aikenflow-generated.json`
manifests. Do not hand-edit committed golden outputs unless the generator change
is intentional and reviewed.
