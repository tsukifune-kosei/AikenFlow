# Changelog

All notable AikenFlow changes are grouped by product boundary so release review
can check CLI behavior, compiler semantics, generators, assurance output, and
agent support independently.

## Unreleased

### CLI

- Added `outline` for optional `ast-outline` repository maps.
- Added `agent-context` for Codex-oriented `.aikenflow/agent-context.md`.
- Added `draft-protocol` for review-only existing-project protocol draft reports.
- Added `export` for frontend-ready JSON bundles containing protocol source,
  graph data, diagnostics, metrics, invariant status, and generated artefacts.
- Added `.aikenflow-generated.json` generation manifests so repeated `gen` runs
  remove stale generated files without deleting user-owned files.
- Changed `gen tests --out tests` to write test files directly into the output
  root instead of nesting `tests/tests`.
- Added internal guards against unsafe or duplicate generated file paths.
- Kept `check`, `gen`, `graph`, and `audit` driven by explicit protocol specs.
- Added CLI integration tests for high-value command paths.

### IR And Parser

- Added stricter YAML parsing for duplicate declarations and unknown fields.
- Added validation diagnostics for unsafe or ambiguous protocol shapes.
- Added validation for output value assignments and output datum field assignments.
- Added validation for empty or duplicate generated input aliases.
- Added validation for generated constraint type compatibility.
- Added warnings for creation outputs whose asset values cannot be inferred.
- Added validation for undeclared output/mint/burn assets and mint/burn policy
  mismatches.
- Added validation that mint/burn effects cannot target lovelace assets.
- Added validation that protocol, state, transition, datum field, and input alias
  names are safe for generated identifiers.
- Added validation that generated identifiers do not normalize to common Aiken or
  TypeScript reserved words.
- Added validation for state, transition, and datum field names that collide
  after generated identifier normalization.
- Added validation for input/output builder parameter prefix collisions inside
  transitions.
- Added validation for unresolved or type-incompatible `datum_field_equals`
  constraints.
- Restricted `signed_by` generated checks to key-hash-like datum fields.
- Kept topology derivation in `aikenflow-ir`.

### Generators

- Stabilized Aiken scaffold output with golden tests.
- Generated `positive` Aiken checks now reject zero with `> 0`.
- Stabilized Lucid-compatible TypeScript builder output with golden tests.
- Applied `outputs[].datum` overrides when generating off-chain output datum values.
- Used input aliases as off-chain builder parameter prefixes.
- Derived off-chain signer parameters from `signed_by` expressions instead of
  emitting one ambiguous `signer` field for every transition.
- Generated self-transition off-chain outputs with `next<State>` address and
  datum parameters so consumed and produced datum values are distinct.
- Carried consumed UTxO assets into off-chain state-transition outputs when an
  output omits explicit `value` assignments.
- Generated off-chain builders now fail clearly when reference-input or
  mint/burn transitions require SDK methods that are not available.
- Added generated README files for contract and off-chain artefacts.
- Kept unsupported semantics visible instead of silently generating false checks.

### Assurance

- Stabilized state graph, topology JSON, invariant matrix, adversarial cases, and
  audit report output.
- Added machine-readable `coverage.json` for constraint and invariant coverage
  summaries.
- Marked unsupported constraints as manual-review coverage.
- Stopped claiming generated validator coverage for creation transitions that do
  not consume script state.
- Stopped claiming generated validator coverage when constraint expressions do
  not resolve to consumed datum fields.
- Marked every audit invariant as not formally proven with explicit partial test
  coverage.
- Added golden tests for high-value assurance artefacts.

### Export

- Added `aikenflow-export` as the reusable Rust-to-frontend bundle layer for the
  future visual protocol cockpit.
- Exported invariant status as partial or missing in the MVP; no invariant is
  marked covered until a stronger proof or test backend exists.
- Made exported transition and protocol risk levels conservative: off-chain-only
  parameter constraints are at least medium risk, while unsupported/custom
  semantics are high risk.
- Included generated Aiken, Lucid, test, and audit artefacts with language
  metadata for file-tree and code-viewer UI surfaces.

### VS Code Extension

- Added the `extensions/vscode-aikenflow` VS Code extension as the v0.1 product
  shell, with the Rust CLI remaining the semantic authority.
- Added command palette actions for analysing protocols, opening the protocol
  cockpit, generating artefacts, exporting audit reports, selecting the backend
  binary, and creating Codex agent context.
- Added real CLI subprocess orchestration, protocol file discovery, safe
  workspace-relative output path validation, status bar summaries, and VS Code
  diagnostics mapping.
- Added a React webview cockpit that renders exported bundle data for graph,
  assurance, findings, generated artefacts, and audit preview surfaces.
- Added webview CSP handling and message validation so the webview can request
  refreshes and file reveals without direct filesystem access.
- Added Extension Host smoke tests and VSIX packaging.

### Agent Support

- Added optional subprocess wrapper for `ast-outline`.
- Added heuristic scanning for Aiken validators, datum/redeemer-like types,
  tests, blueprints, generated artefacts, and off-chain transaction builders.
- Added compact agent context and protocol draft markdown artefacts.
- Added draft protocol sketching for inferred Aiken record fields.
- Added Codex-oriented suggested edit points with file and line hints.
- Filtered recognized `ast-outline` file blocks to keep declarations and line
  ranges without source bodies.
- Strengthened protocol draft warnings with `THIS IS A DRAFT. NOT VERIFIED.`
- Kept repository intelligence separate from compiler correctness.

### Documentation

- Added architecture, protocol spec, execution plan, coding style, and release
  checklist documentation.
- Added frontend architecture and UI/UX design documentation for the future
  AikenFlow visual protocol cockpit.
- Added GitHub Actions CI, release packaging, issue templates, security policy,
  contribution guide, root Apache-2.0 license, and release smoke scripts.
