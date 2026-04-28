# AikenFlow

[![CI](https://github.com/tsukifune-kosei/AikenFlow/actions/workflows/ci.yml/badge.svg)](https://github.com/tsukifune-kosei/AikenFlow/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

AikenFlow is an Aiken-compatible protocol compiler and assurance toolkit for multi-step Cardano eUTxO applications.

AikenFlow v0.1 is not a formal verification tool. It generates assurance
artefacts and review surfaces from explicit protocol IR; unsupported
constraints and partial invariant coverage still require manual review.

It treats the protocol, not a single validator, as the compilation unit:

```text
protocol.yaml
  -> Semantic Protocol IR
  -> Aiken validator scaffolds
  -> Lucid-compatible TypeScript transaction builders
  -> State graphs, topology JSON, invariant matrices, adversarial test stubs, audit reports
```

This repository contains the v0.1 technical preview: a Rust CLI backend plus a
VS Code extension protocol cockpit.

## Status

Implemented:

- YAML protocol specification parser.
- Semantic IR and high-level diagnostics.
- `aikenflow` CLI with `init`, `check`, `graph`, `gen`, and `audit`.
- Aiken validator scaffold generation for spending transitions.
- Lucid-compatible TypeScript builder generation.
- Mermaid state graph generation.
- Topology JSON generation.
- Invariant matrix, adversarial case list, test stubs, and audit artefact generation.
- Frontend JSON bundle export for the visual protocol cockpit.
- VS Code extension under `extensions/vscode-aikenflow/` with real CLI
  subprocess commands, status bar integration, diagnostics mapping, Extension
  Host smoke tests, VSIX packaging, and React webview cockpit rendering.
- Optional `ast-outline` repository maps and Codex agent context generation.
- Existing-project protocol draft reports for manual review.
- Starter protocols: simple vault, auction-lite, programmable-token-mini.

Not implemented yet:

- Full Cardano transaction balancing.
- Formal proof backend.
- Verified reverse engineering from existing Aiken projects.
- Multiple off-chain adapters beyond the Lucid-compatible structural adapter.
- End-to-end Yaci Dev Kit runner.

## Quickstart

Install the CLI from a GitHub Release once tagged, or build it locally:

```bash
cargo install --path crates/aikenflow-cli
```

```bash
cargo run -p aikenflow-cli -- init simple-vault
cd simple-vault
cargo run -p aikenflow-cli --manifest-path ../Cargo.toml -- check protocol.yaml
cargo run -p aikenflow-cli --manifest-path ../Cargo.toml -- gen all protocol.yaml --out generated
```

From the repository root, you can also run against included examples:

```bash
cargo run -p aikenflow-cli -- check examples/simple-vault/protocol.yaml
cargo run -p aikenflow-cli -- graph examples/simple-vault/protocol.yaml
cargo run -p aikenflow-cli -- gen all examples/simple-vault/protocol.yaml --out examples/simple-vault/generated
```

Run the VS Code extension build and unit checks:

```bash
cd extensions/vscode-aikenflow
npm install
npm run check
```

Run the full VS Code extension gate, including Extension Host smoke test and
VSIX packaging:

```bash
cd extensions/vscode-aikenflow
npm run check:full
```

## VS Code Extension

Install the VSIX from a GitHub Release, or build it locally:

```bash
cd extensions/vscode-aikenflow
npm ci
npm run vsix
code --install-extension vscode-aikenflow-0.1.1.vsix
```

1. Install or build the `aikenflow` CLI.
2. Set `aikenflow.backendPath` if the binary is not on `PATH`.
3. Open a workspace containing `protocol.yaml`, `protocol.yml`,
   `aikenflow.yaml`, or `aikenflow.yml`.
4. Run `AikenFlow: Analyse Protocol`.
5. Run `AikenFlow: Open Protocol Cockpit`.

For local extension development, point VS Code at the built backend:

```json
{
  "aikenflow.backendPath": "/absolute/path/to/AikenFlow/target/debug/aikenflow"
}
```

## CLI

```bash
aikenflow init simple-vault
aikenflow init auction-lite
aikenflow init programmable-token-mini

aikenflow check protocol.yaml
aikenflow check protocol.yaml --json

aikenflow graph protocol.yaml
aikenflow graph protocol.yaml --format json

aikenflow gen all protocol.yaml --out generated
aikenflow gen aiken protocol.yaml --out contracts
aikenflow gen lucid protocol.yaml --out offchain
aikenflow gen assurance protocol.yaml --out assurance
aikenflow gen tests protocol.yaml --out tests

aikenflow audit protocol.yaml --out AUDIT.md
aikenflow export protocol.yaml --out public/examples/simple-vault.json

aikenflow outline .
aikenflow agent-context . --for codex
aikenflow draft-protocol .
```

Generated output roots include `.aikenflow-generated.json`. Later `gen` runs use
that manifest to remove stale generated files without deleting user-owned files
and to refuse overwriting generated files that were manually edited.
`gen tests --out tests` writes test files directly into `tests/`, not
`tests/tests/`.

## AI Agent Support

AikenFlow can optionally use `ast-outline` to generate compact repository maps for Codex and other AI coding agents.
This reduces token usage and helps agents patch exact files without reading entire repositories.
Outline output keeps file paths, declarations, signatures, and line ranges; source bodies are filtered from recognized outline blocks.

This layer is deliberately separate from compiler correctness:

```text
ast-outline = navigation / code map / agent context
AikenFlow IR = protocol semantics / invariants / generation
```

Install or configure `ast-outline` so it is available on `PATH`, or set:

```bash
export AIKENFLOW_AST_OUTLINE=/path/to/ast-outline
```

Commands:

```bash
aikenflow outline <path>
aikenflow agent-context <path> --for codex
```

`agent-context` writes `.aikenflow/agent-context.md` under the target project.

`draft-protocol` writes `.aikenflow/protocol-draft.md` by default. The result is labeled `THIS IS A DRAFT. NOT VERIFIED.` It can point at candidate validators, datum/redeemer-like types, inferred datum fields, and off-chain builders, but it is not a protocol spec and cannot be used as compiler evidence until a human rewrites it into `protocol.yaml` and runs `aikenflow check`.

## Repository Layout

```text
AikenFlow/
  crates/
    aikenflow-ir/          Semantic protocol model and diagnostics
    aikenflow-parser/      YAML parser into IR
    aikenflow-aiken-gen/   Aiken validator scaffold generator
    aikenflow-lucid-gen/   Lucid-compatible TypeScript builder generator
    aikenflow-assurance/   Graphs, topology, audit, invariant/test artefacts
    aikenflow-export/      Frontend JSON bundle exporter
    aikenflow-agent-support/ Optional ast-outline and agent context layer
    aikenflow-cli/         CLI entrypoint
  examples/
    simple-vault/
    auction-lite/
    programmable-token-mini/
  extensions/
    vscode-aikenflow/        VS Code extension and React webview cockpit
  docs/
  tests/golden/
```

## Protocol Spec Shape

```yaml
protocol: SimpleVault

assets:
  - name: ada
    kind: lovelace

states:
  Locked:
    datum:
      owner: PubKeyHash
      amount: Lovelace
      deadline: POSIXTime

transitions:
  Deposit:
    inputs: []
    outputs:
      - state: Locked
        value:
          ada: "$amount"
    constraints:
      - signed_by: "$owner"
      - positive: "$amount"

  Withdraw:
    inputs:
      - state: Locked
    outputs: []
    constraints:
      - signed_by: "datum.owner"
      - after: "datum.deadline"

invariants:
  - name: withdrawal_requires_owner
    expression: "transition.Withdraw requires signature(datum.owner)"
```

## Development

Project execution and implementation rules live in:

- `docs/execution-plan.md`
- `docs/coding-style.md`
- `docs/vscode-extension-architecture.md`
- `docs/release-checklist.md`
- `CHANGELOG.md`

```bash
cargo fmt --all
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p aikenflow-cli -- check examples/simple-vault/protocol.yaml
```

Release candidates also run:

```bash
cargo audit
cd extensions/vscode-aikenflow && npm audit --audit-level=moderate && npm run check:full
bash scripts/verify-examples.sh
```

`scripts/verify-examples.sh` requires the Aiken CLI. CI pins Aiken `v1.1.19`
and checks generated Aiken contracts with `aiken check`.

The generated Aiken project follows the current Aiken validator layout:
application validators live in `validators/`, with an `aiken.toml` manifest and
stdlib dependency.
