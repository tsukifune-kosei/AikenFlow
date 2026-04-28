# Architecture

AikenFlow is split into small crates so each compiler responsibility is explicit.

```text
Protocol YAML
  -> aikenflow-parser
  -> aikenflow-ir
  -> aikenflow-aiken-gen
  -> aikenflow-lucid-gen
  -> aikenflow-assurance
  -> aikenflow-export
  -> aikenflow-cli
```

## IR First

The semantic IR is the stable center of the product. Generators should not parse YAML directly. They consume `Protocol`, `StateDecl`, `TransitionDecl`, `Constraint`, and `InvariantDecl` from `aikenflow-ir`.

This keeps the product extensible:

- Add a future TOML or DSL parser without changing generators.
- Add Mesh, Blaze, or Atlas adapters without changing YAML parsing.
- Add richer diagnostics without coupling them to one output format.
- Add formal-export backends later without redesigning protocol specs.

## Current Generation Strategy

The MVP uses full scaffold generation for simple spending validators:

- One generated Aiken validator module per consumed state.
- One redeemer constructor per transition consuming that state.
- Generated checks for `signed_by`, `before`, `after`, `positive`, and simple datum equality constraints.
- Unsupported constraints remain visible in audit and topology artefacts.

The production path should add hook-based generation so teams can keep hand-written Aiken predicates while AikenFlow owns topology, off-chain builders, tests, and audit artefacts.

## Diagnostics

Diagnostics are protocol-level:

- Unknown state references.
- Duplicate state or transition names.
- Missing transitions or invariants.
- Unknown constraints.
- Owner-like datum fields consumed without any signature constraint.

Diagnostics should stay closer to the protocol model than to generated code syntax.

## Frontend Export Boundary

`aikenflow-export` turns validated protocol IR, generated artefacts, diagnostics,
graph data, invariant status, and review metrics into a stable JSON bundle for
the frontend workbench. The production frontend direction is a VS Code extension,
documented in `docs/vscode-extension-architecture.md`.

The exporter does not own protocol meaning. It packages the same IR-driven
compiler outputs that `check`, `graph`, `gen`, and `audit` use. Repository
outlines and agent-context files remain outside this semantic path.

## Agent Support Boundary

`aikenflow-agent-support` is an optional repository-intelligence layer. It can call `ast-outline`, scan existing Aiken/Cardano projects, generate `.aikenflow/agent-context.md` for Codex, and write explicit protocol draft reports for manual review.

It must not participate in compiler correctness:

```text
ast-outline = navigation / code map / agent context
AikenFlow IR = protocol semantics / invariants / generation
```

This boundary matters because an AST outline or repository heuristic can help locate validators, datum/redeemer-like types, off-chain builders, generated artefacts, and candidate protocol draft material, but it is not a trusted source for protocol invariants or transition semantics.

## Governance Documents

Implementation sequencing is tracked in `docs/execution-plan.md`.

Coding style, crate ownership, error handling, generated artefact rules, and test gates are normative in `docs/coding-style.md`.
