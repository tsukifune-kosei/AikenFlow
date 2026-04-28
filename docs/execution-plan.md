# AikenFlow Execution Plan

This plan keeps the product shape complete while allowing workstreams to move in
parallel. Stages define readiness boundaries, not serial blockers.

## Operating Model

The core product remains:

```text
protocol spec
  -> semantic IR
  -> Aiken generation
  -> off-chain tx builder generation
  -> state graph
  -> invariant/test/audit artefacts
```

The optional intelligence layer remains:

```text
repo / existing Aiken project
  -> compact AST outline
  -> validator/type/function map
  -> agent context file
```

The two layers must stay separate:

```text
ast-outline = navigation / code map / agent context
AikenFlow IR = protocol semantics / invariants / generation
```

## Stage 0: Foundation Lock

Goal: make the project easy to extend without changing ownership rules each time.

Parallel work:

- Keep crate responsibilities explicit in `docs/architecture.md`.
- Enforce implementation rules through `docs/coding-style.md`.
- Keep README command examples aligned with implemented CLI behavior.
- Maintain workspace-level quality gates.

Exit criteria:

- Coding style document exists and is linked.
- Architecture names the compiler boundary and agent-support boundary.
- `cargo fmt`, `cargo test`, and `cargo clippy` pass.

## Stage 1: Protocol IR And Parser Hardening

Goal: make the protocol document a stable semantic source.

Parallel work:

- Expand IR validation diagnostics for ambiguous or unsafe protocol shapes.
- Add parser tests for missing fields, duplicate declarations, unknown states, and unsupported constraints.
- Keep topology derivation in IR, not in generators.
- Document accepted YAML shape in `docs/protocol-spec.md`.

Exit criteria:

- All generator crates can rely on validated IR.
- Parser errors and semantic diagnostics are distinct.
- Invalid protocol examples have regression coverage.

## Stage 2: Aiken Generation Shape

Goal: generate inspectable Aiken projects that match current Aiken conventions.

Parallel work:

- Keep standalone contract output under `generated/contracts`.
- Emit `aiken.toml`, `validators/*.ak`, and contract README consistently.
- Use deterministic datum, redeemer, validator, and function naming.
- Keep unsupported constraints visible in audit artefacts.

Exit criteria:

- Golden tests cover generated validator shape.
- Generated Aiken does not depend on repository-intelligence output.
- Aiken generator behavior is documented as scaffold generation, not formal verification.

## Stage 3: Off-chain Builder Generation

Goal: generate useful off-chain transaction builder scaffolds without locking the
project to one SDK forever.

Parallel work:

- Keep Lucid-compatible structural types as the default adapter surface.
- Isolate adapter-specific assumptions behind generated types and builder interfaces.
- Reserve Mesh, Blaze, and Atlas as future adapters.
- Add tests for transition-to-builder mapping and transaction marker output.

Exit criteria:

- Generated TypeScript is deterministic.
- Builders expose typed parameter objects.
- SDK-specific side effects are not hidden in generated functions.

## Stage 4: Assurance Artefacts

Goal: make the protocol reviewable by humans and future tools.

Parallel work:

- Keep state graph, topology JSON, invariant matrix, adversarial case list, and audit report generated from IR.
- Stabilize markdown headings for downstream tooling.
- Add golden tests for high-value artefacts.
- Clearly mark unsupported or assumption-heavy constraints.

Exit criteria:

- Assurance output can be regenerated without noisy diffs.
- Audit artefacts explain limitations instead of implying proofs.
- Tests cover terminal states and transition topology.

## Stage 5: Agent Support Layer

Goal: reduce repository navigation cost for Codex and similar agents.

Parallel work:

- Keep `aikenflow outline <path>` as a compact map command.
- Keep `aikenflow agent-context <path> --for codex` as the context-file generator.
- Parse `ast-outline` output only as a best-effort map.
- Use heuristics for Aiken validators, datum/redeemer-like types, tests, blueprints, and off-chain builder files.

Exit criteria:

- Missing `ast-outline` fails with a clear configuration message.
- Empty, Aiken-only, and mixed Aiken/TypeScript projects have tests.
- Generated `.aikenflow/agent-context.md` stays concise and stable.

## Stage 6: Existing Project Analysis Drafting

Goal: let AikenFlow assist with existing repositories without pretending it has
derived a verified protocol.

Parallel work:

- Use agent support to locate candidate validators and builders.
- Produce draft protocol suggestions only as explicit, reviewable artefacts.
- Keep `aikenflow draft-protocol <path>` labeled as a draft report, not a compiler input.
- Require a user-owned protocol spec before normal `check`, `gen`, `graph`, or `audit` semantics apply.
- Keep unknown files and unsupported shapes visible in warnings.

Exit criteria:

- Existing-project analysis cannot bypass IR validation.
- Draft output is labeled as a draft.
- The compiler path remains deterministic and spec-driven.

## Stage 7: VS Code Extension Frontend

Goal: make AikenFlow usable inside the editor Aiken/Cardano developers already
use, without building a standalone IDE.

Parallel work:

- Add `extensions/vscode-aikenflow` as the only production frontend path.
- Keep the extension host as orchestration only: commands, diagnostics, status
  bar, webview lifecycle, output channel, and safe file writes.
- Run real Rust CLI commands through a subprocess wrapper; do not use mock
  analysis or webview-only protocol inference.
- Render the cockpit through a React webview using the exported JSON bundle.
- Publish diagnostics into VS Code Problems only from compiler output and real
  source ranges.

Exit criteria:

- `AikenFlow: Analyse Protocol` runs `aikenflow export` against a real
  `protocol.yaml` and opens the cockpit from `.aikenflow/bundle.json`.
- `AikenFlow: Generate Artefacts` runs `aikenflow gen all` and protects local
  edits through the generated manifest.
- `AikenFlow: Export Audit Report` writes real audit markdown.
- Missing compiler binary, invalid protocol, empty workspace, and CLI failure
  produce clear VS Code errors.
- Extension integration tests cover the primary workflow.

## Stage 8: Release Quality

Goal: make each release auditable.

Parallel work:

- Keep changelog entries grouped by CLI, IR, generators, assurance, and agent support.
- Run all workspace quality gates.
- Smoke-test included examples.
- Regenerate golden outputs intentionally.
- Run VS Code extension smoke tests once `extensions/vscode-aikenflow` exists.

Exit criteria:

- No undocumented command behavior.
- No generated artefact drift.
- No clippy warnings.
- README quickstart works from a clean checkout.
- VS Code extension command paths run real compiler commands, not fixture-only data.

## Non-blocking Workstream Map

These tracks can move independently once Stage 0 is in place:

- IR/parser hardening.
- Aiken generator output.
- Off-chain generator output.
- Assurance artefacts.
- Agent-support repository maps.
- VS Code extension shell and webview.
- Documentation and examples.

Integration happens through stable contracts:

- Parser outputs `Protocol`.
- IR validates and derives topology.
- Generators consume validated IR.
- Agent support produces context only.
- CLI coordinates commands and file IO.
- VS Code extension consumes CLI/export output and does not own protocol meaning.

## Default Verification Matrix

Run the full gate before considering a multi-crate change complete:

```bash
cargo fmt --all
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
```

For CLI changes, also run at least one example smoke test:

```bash
cargo run -p aikenflow-cli -- check examples/simple-vault/protocol.yaml
cargo run -p aikenflow-cli -- graph examples/simple-vault/protocol.yaml
cargo run -p aikenflow-cli -- outline examples/simple-vault
```

Once `extensions/vscode-aikenflow` exists, also run the extension smoke test
documented in `docs/vscode-extension-architecture.md`.
