# AikenFlow Frontend Architecture & UI/UX

Status: production frontend direction.

AikenFlow ships as a VS Code extension with a React webview protocol cockpit.
It does not ship a standalone IDE or a standalone dashboard in v0.1.

## Product Positioning

AikenFlow for VS Code is a protocol assurance workbench for Aiken/Cardano
builders.

It is not a no-code contract builder and it is not a replacement for the Aiken
language extension. AikenFlow works one level above validator editing:

```text
protocol.yaml
  -> Rust compiler core
  -> JSON bundle
  -> VS Code diagnostics + React webview cockpit
```

Core message:

```text
Compile protocols, not just validators.
```

## Frontend Boundary

```text
Rust CLI = semantic authority
VS Code extension host = orchestration
React webview = visual protocol cockpit
```

The frontend must not parse protocol semantics, infer invariants, generate
Aiken, or decide compiler correctness. It calls real CLI commands and renders
compiler-owned outputs.

## User Workflow

```text
1. Open an Aiken/Cardano workspace in VS Code.
2. Open or select protocol.yaml.
3. Run AikenFlow: Analyse Protocol.
4. The extension runs check/export through the Rust CLI.
5. VS Code Problems shows source diagnostics with real ranges.
6. The cockpit opens beside the editor.
7. The user inspects graph topology, risks, invariants, generated artefacts,
   audit output, and source-linked findings.
```

## VS Code Surface

The extension owns these VS Code integration points:

- Command Palette commands.
- Explorer and editor context menu commands for `protocol.yaml`.
- Status bar item with real analysis state.
- Output Channel for CLI stdout/stderr.
- Problems diagnostics from compiler output only.
- Webview panel for the protocol cockpit.
- Workspace settings for compiler path and output locations.

The VS Code editor remains the source editor. The webview should not embed a
full YAML editor.

## Webview Layout

Default cockpit layout:

```text
┌────────────────────────────────────────────────────────────┐
│ Protocol Summary: name · states · transitions · invariants │
├───────────────────────────────────────┬────────────────────┤
│ Protocol Graph                        │ Assurance          │
│ state/transaction topology            │ coverage/risks     │
│ selected path highlights              │ invariant matrix   │
├───────────────────────────────────────┴────────────────────┤
│ Generated Artefacts / Audit Preview / Topology JSON        │
└────────────────────────────────────────────────────────────┘
```

The cockpit should feel like a protocol risk surface, not a decorative demo.

## Core Interactions

- Click a state: show datum fields, consumed-by transitions, produced-by
  transitions, and a source button.
- Click a transition: show consumed states, produced states, constraints,
  generated artefacts, warnings, and a source button.
- Click an invariant: show status, related transitions, warnings, and a source
  button.
- Click a finding: reveal the related source range when available.
- Click an artefact: open generated file or preview at a relevant line.

All navigation messages must be typed and validated before use.

## Visual Direction

Use a restrained dark IDE-native cockpit:

- Background follows VS Code theme variables.
- Graph states use blue/cyan.
- Normal transitions use blue/cyan.
- Warnings use yellow/orange.
- Missing or high-risk findings use red.
- Generated artefact relations use purple.
- Code uses VS Code editor font variables.

Do not use marketing-style hero sections, fake progress, decorative blobs, or
standalone landing-page patterns.

## Data Contract

The webview consumes the bundle emitted by:

```bash
aikenflow export protocol.yaml --out .aikenflow/bundle.json
```

The bundle must include protocol metadata, graph states/transitions,
diagnostics, invariants, metrics, generated artefacts, and audit content. The
extension validates the bundle shape before posting it to the webview.

## Command UX

Required commands:

```text
AikenFlow: Analyse Protocol
AikenFlow: Open Protocol Cockpit
AikenFlow: Generate Artefacts
AikenFlow: Export Audit Report
AikenFlow: Create Agent Context
AikenFlow: Select Backend Binary
```

Every command must run a real Rust CLI operation or open a real generated file.
No production command may use mock analysis, fabricated artefacts, or hardcoded
demo bundles.

## Generated File Safety

Generated writes happen through the CLI. The frontend must respect the generated
manifest and surface overwrite conflicts clearly. If a user modified a generated
file, AikenFlow must refuse to overwrite or remove it until the user resolves
the conflict.

## Agent Support

Agent support remains optional and outside compiler semantics:

```text
ast-outline = navigation / code map / agent context
AikenFlow IR = protocol semantics / invariants / generation
```

The VS Code command for agent context runs:

```bash
aikenflow agent-context <workspace> --for codex
```

and opens `.aikenflow/agent-context.md`.

## Accessibility And Performance

- Keep status and risk information available as text, not only color.
- Keep controls reachable through keyboard focus.
- Avoid long-running work in the extension host event loop.
- Use CLI subprocess cancellation when VS Code cancels progress.
- Keep webview assets bundled and CSP-restricted.
- Avoid reading workspace files directly from the webview.

## Testing

Required frontend gates:

```bash
cd extensions/vscode-aikenflow
npm run check
npm run check:full
```

Coverage must include:

- unit tests for source and artefact navigation
- diagnostics parsing
- generated manifest conflict detection
- typed webview message handling
- Extension Host smoke test that runs the real CLI on a fixture workspace
- VSIX packaging verification

## Non-Goals

Do not build these for v0.1:

- standalone Cardano IDE
- alternative IDE shell
- standalone hosted dashboard
- full graph editor
- wallet integration
- formal proof UI
- no-code contract builder copy

## Acceptance Criteria

The frontend is ready when a user can install the extension, configure the
compiler path, run analysis on a real `protocol.yaml`, see diagnostics in
Problems, inspect the protocol cockpit, generate artefacts, export the audit
report, create Codex agent context, and package a clean VSIX without test or
prototype artefacts.
