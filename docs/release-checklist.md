# Release Checklist

Use this checklist before tagging or publishing an AikenFlow release.

## Quality Gate

Run the workspace gate from the repository root:

```bash
cargo fmt --all -- --check
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo audit
```

For the VS Code extension, run dependency and packaging checks from
`extensions/vscode-aikenflow`:

```bash
npm audit --audit-level=moderate
npm run check:full
unzip -l vscode-aikenflow-<version>.vsix
```

The VSIX must contain compiled extension code, the built webview assets,
`package.json`, README, and license files only. It must not contain
`node_modules`, source fixtures, test output, or workspace-local generated
artefacts.

CI must run the deterministic non-GUI gates on pushes and pull requests through
`.github/workflows/ci.yml`: Rust format, tests, clippy, audit, npm audit,
webview build, extension unit tests, integration-test compilation, backend
fixture smoke, VSIX packaging, VSIX archive verification, and example smoke
tests. The full VS Code Extension Host smoke remains a local/manual release
gate through `npm run check:full` because it launches a desktop Electron host.

## Example Smoke Tests

Run the included examples through the CLI:

```bash
cargo run -p aikenflow-cli -- check examples/simple-vault/protocol.yaml
cargo run -p aikenflow-cli -- graph examples/simple-vault/protocol.yaml
cargo run -p aikenflow-cli -- gen all examples/simple-vault/protocol.yaml --out /tmp/aikenflow-simple-vault-generated
cargo run -p aikenflow-cli -- audit examples/simple-vault/protocol.yaml --out /tmp/aikenflow-simple-vault-AUDIT.md
cargo run -p aikenflow-cli -- export examples/simple-vault/protocol.yaml --out /tmp/aikenflow-simple-vault.bundle.json
cargo run -p aikenflow-cli -- draft-protocol examples/simple-vault --out /tmp/aikenflow-simple-vault-protocol-draft.md
```

For generated off-chain packages, run TypeScript validation after generation:

```bash
cd /tmp/aikenflow-simple-vault-generated/offchain
npm install
npm run check
```

For generated Aiken contracts, install the pinned Aiken release used by CI and
run the full example smoke:

```bash
export PATH="$HOME/.aiken/bin:$PATH"
bash scripts/verify-examples.sh
```

The smoke script runs `check --json`, `export`, `gen all`, `audit`, generated
off-chain `npm run check`, and generated contract `aiken check` for every
included example.

`outline` and `agent-context` require `ast-outline` to be installed or configured
with `AIKENFLOW_AST_OUTLINE`.

## Generated Artefact Review

- Regenerate golden outputs intentionally after generator or assurance changes.
- Review README command examples after CLI changes.
- Confirm draft and agent-support output remains labeled as repository
  intelligence, not compiler semantics.
- Confirm frontend bundle export remains generated from IR and does not use
  repository-intelligence output.
- Confirm `CHANGELOG.md` has entries grouped by CLI, IR/parser, generators,
  assurance, export, agent support, and documentation.

## VS Code Extension Gate

Once `extensions/vscode-aikenflow` exists, every release must also verify:

- `AikenFlow: Analyse Protocol` runs the real Rust CLI and writes
  `.aikenflow/bundle.json`.
- Protocol diagnostics appear in VS Code Problems only when compiler output
  includes real source ranges.
- The React webview renders graph, assurance, inspector, artefacts, and audit
  data from the exported bundle.
- `AikenFlow: Generate Artefacts` uses manifest checks before writing files.
- Workspace output settings are safe relative paths only. Absolute paths,
  parent traversal, empty segments, and NUL bytes must be rejected before any
  CLI write command runs.
- Bundle and webview message validators reject unsafe generated artefact paths.
- `AikenFlow: Export Audit Report` writes real markdown from the Rust CLI.
- `AikenFlow: Create Agent Context` writes `.aikenflow/agent-context.md`
  through the Rust CLI and keeps `ast-outline` as navigation-only context.
- Fixture integration tests copy source fixtures to a temporary workspace so
  stale generated files cannot hide command failures.
- Extension Host tests cover missing backend paths and invalid protocols with
  source-ranged VS Code diagnostics.
- Missing backend binary, invalid protocol, empty workspace, and CLI failure
  paths show actionable errors.
- Extension tests run through the documented VS Code test harness.
- `cd extensions/vscode-aikenflow && npm run check:full` succeeds.

## Release Packaging

Tag releases as `vX.Y.Z`. `.github/workflows/release.yml` must produce:

- native `aikenflow` CLI archives for Linux, macOS Intel, macOS Apple Silicon,
  and Windows;
- SHA-256 checksum files for every archive;
- `vscode-aikenflow-<version>.vsix` and checksum;
- a GitHub Release containing all artefacts.

Marketplace publishing is optional and only runs when `VSCE_PAT` is configured
as a repository secret. The extension publisher in `package.json` must match the
Marketplace publisher that owns the token.

## Boundary Check

Release candidates must preserve this split:

```text
ast-outline / repository heuristics = navigation, code maps, and draft context
AikenFlow IR = protocol semantics, invariants, generation, and assurance
```
