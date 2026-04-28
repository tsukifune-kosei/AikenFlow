# AikenFlow Coding Style

This document is normative for AikenFlow implementation work. Code that conflicts
with this document should be changed before it is extended.

## Core Boundary

AikenFlow has two separate layers:

```text
Protocol spec -> Semantic IR -> generators -> assurance artefacts
Repository files -> ast-outline / heuristics -> navigation context
```

The first line is compiler semantics. The second line is agent support.

Rules:

- `aikenflow-ir` is the only source of protocol semantics after parsing.
- `ast-outline` must never prove invariants, validate transitions, or derive protocol meaning.
- Agent-support heuristics may locate candidate files, validators, types, and transaction builders.
- Existing-project analysis must produce explicit draft artefacts, not compiler-ready semantics.
- Generator output must be derived from `Protocol` and related IR types, not from generated code or repository outlines.
- Any future reverse-engineering feature must produce an explicit protocol draft before semantic validation.

## Crate Responsibilities

Each crate owns one responsibility:

- `aikenflow-ir`: semantic model, diagnostics, topology, validation.
- `aikenflow-parser`: external protocol documents into IR.
- `aikenflow-aiken-gen`: Aiken project and validator scaffolds from IR.
- `aikenflow-lucid-gen`: off-chain TypeScript builders from IR.
- `aikenflow-assurance`: graphs, topology JSON, tests, invariant matrix, audit artefacts from IR.
- `aikenflow-export`: frontend JSON bundle packaging from IR, diagnostics, generators, and assurance output.
- `aikenflow-agent-support`: optional repository maps and agent context files.
- `aikenflow-cli`: argument parsing, file IO orchestration, user-facing commands.
- `extensions/vscode-aikenflow`: VS Code orchestration, diagnostics publishing,
  status bar, webview lifecycle, and safe workspace writes.

Rules:

- Generators must not parse YAML, JSON protocol specs, or repository source files.
- Parser code must not generate Aiken, TypeScript, or assurance artefacts.
- CLI code may coordinate crates but should not contain semantic rules.
- VS Code extension code may coordinate CLI commands and render bundle output,
  but must not parse protocol semantics, compute invariants, or generate
  contracts.
- Shared behavior belongs in the smallest crate that owns the concept.
- Do not add cross-crate dependencies that create cycles or blur ownership.

## VS Code Extension Style

The production frontend is a thin VS Code extension over the Rust compiler.

Rules:

- Extension commands must call real `aikenflow` CLI commands or report a clear
  missing-compiler error.
- Production command paths must not use mock analysis, fake progress, or
  hard-coded generated artefacts.
- Webview UI must render compiler-exported bundles; it must not read workspace
  files directly.
- Webview-to-extension communication must use typed message unions.
- Diagnostics must come from compiler output and must not invent source ranges.
- Use `execFile` or `spawn` with argument arrays; do not shell-interpolate
  workspace paths.
- Workspace writes must go through manifest-aware helpers and must not silently
  overwrite user-authored files.
- Webview HTML must use a restrictive content security policy and nonced local
  scripts.

## Rust Style

Baseline:

- Rust edition is workspace-controlled.
- Run `cargo fmt --all` before completing any change.
- Run `cargo clippy --workspace --all-targets -- -D warnings` before completing non-trivial changes.
- Prefer plain data structures and pure functions over hidden global state.

Public APIs:

- Public semantic structs should derive `Debug`, `Clone`, `PartialEq`, and `Eq` when practical.
- Use typed enums for domain concepts. Do not model protocol semantics with unbounded strings unless the spec itself is free-form.
- Accept `&Path` for input paths and return `PathBuf` when ownership is needed.
- Prefer `BTreeMap` and `BTreeSet` when output order affects tests, generated files, markdown, or CLI output.

Implementation:

- Keep functions short enough that one responsibility is obvious.
- Use helper functions for formatting and deterministic ordering.
- Avoid premature traits. Add traits only when there are at least two real implementations or a strong test boundary.
- Avoid macros unless they remove meaningful repetition without hiding behavior.
- Do not use `unsafe`.
- Do not use runtime network calls in compiler, generator, or assurance code.

Panics:

- Production code must not use `unwrap`, `expect`, or `panic` for recoverable input, filesystem, parsing, or subprocess errors.
- Tests may use `expect` for fixture setup.
- Generated tests must not pass without real coverage. If an emulator fixture is
  not wired yet, generated cases must fail loudly or be clearly marked as
  pending instead of producing a false green result.

Comments:

- Comments should explain non-obvious intent or domain constraints.
- Do not add comments that restate the next line of code.
- Boundary comments are acceptable where they prevent semantic misuse.

## Error Handling

Library crates:

- Use crate-specific error types when callers can act on failures.
- Use `thiserror` for structured errors.
- Preserve source errors with `#[source]` when wrapping IO, parser, or subprocess errors.
- Error messages should name the relevant file, field, command, or semantic object.

CLI:

- Use `anyhow` for command-level orchestration.
- Add context at file and command boundaries.
- User-facing failures should say what failed and what action can fix it.

Diagnostics:

- Protocol diagnostics are data, not strings alone.
- Diagnostics must carry severity and target context when available.
- Fatal compiler behavior should be driven by diagnostic severity, not by searching message text.

## Generated Artefacts

Generated artefacts must be deterministic:

- Stable file paths.
- Stable declaration ordering.
- Stable markdown headings.
- Stable JSON key ordering where practical.
- No timestamps, machine-specific absolute paths, random IDs, or environment-dependent output.

Aiken generation:

- Generated validators live under `validators/`.
- Generated projects include `aiken.toml` when emitting a standalone contract project.
- Datum and redeemer names must come from IR naming helpers.
- Unsupported constraints must remain visible in audit artefacts instead of silently disappearing.

TypeScript generation:

- Generated off-chain code should be adapter-shaped and easy to replace with real Lucid, Mesh, or Blaze imports.
- Transaction builder functions should expose typed parameter objects.
- Do not generate hidden network calls, wallet selection, or signing side effects.

Markdown artefacts:

- Use predictable heading names.
- Use fenced code blocks with language tags where applicable.
- Keep audit and agent-context files concise enough for human review and AI context use.

Generated write safety:

- CLI generation must track generated files in `.aikenflow-generated.json`.
- Generated manifests must include content hashes for files owned by AikenFlow.
- Regeneration must refuse to overwrite or remove files whose current content no
  longer matches the previous manifest hash.
- Existing untracked files must not be overwritten by generation.

## Agent Support

`aikenflow-agent-support` is allowed to:

- Call `ast-outline` through a subprocess wrapper.
- Parse stable or best-effort outline text into compact symbols.
- Scan repository files using explicit heuristics.
- Generate `.aikenflow/agent-context.md`.
- Generate `.aikenflow/protocol-draft.md` as a review-only existing-project analysis artefact.

It is not allowed to:

- Validate protocol transitions.
- Prove invariants.
- Decide generated validator semantics.
- Modify user source files.
- Require `ast-outline` for normal compile, check, gen, graph, or audit commands.

Subprocess rules:

- Do not vendor `ast-outline`.
- Use `AIKENFLOW_AST_OUTLINE` as the override path.
- If the executable is missing, fail with a clear installation/configuration message.
- Treat unstable `ast-outline` output as raw context first; add structured parsing only when tests cover the observed format.

## CLI Output

CLI output should be script-friendly:

- Successful commands print concise status or the requested artefact.
- Errors must be actionable.
- Do not print debug structs in normal output.
- JSON output must be valid JSON with no extra prose.
- Markdown output must be valid Markdown with stable headings.

Command rules:

- `check` validates explicit protocol specs.
- `gen`, `graph`, `audit`, and `export` load IR and fail on validation errors.
- `outline` and `agent-context` are optional intelligence commands and may fail if `ast-outline` is unavailable.
- `draft-protocol` is a review-only command. Its output must say that a human-owned protocol spec is required before compiler commands.

## Tests

Test expectations:

- Unit tests cover small semantic rules and formatting helpers.
- Integration tests cover CLI-level behavior and cross-crate contracts.
- Golden tests cover generated artefacts whose output must remain stable.
- Every bug fix should add a regression test unless the change is documentation-only.

Test style:

- Test names should describe observable behavior.
- Fixtures should be minimal and local to the test unless reused across multiple crates.
- Tests may use `expect` for setup, but assertions should explain the behavior being checked.
- Avoid snapshot sprawl. Prefer focused golden files for compiler output that users inspect.

Required quality gate:

```bash
cargo fmt --all
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
```

## Documentation

Documentation is part of the product contract:

- Update README when adding or changing CLI commands.
- Update architecture docs when changing crate boundaries.
- Update protocol spec docs when changing accepted YAML shape.
- Update this file when implementation rules change.

Docs must distinguish implemented behavior from planned behavior. Do not describe
future capabilities as available commands or supported semantics.
