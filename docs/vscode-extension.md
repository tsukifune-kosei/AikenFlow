# VS Code Extension

AikenFlow for VS Code is a thin IDE integration over the Rust backend.

```text
Rust backend = protocol semantics, validation, generation, assurance
VS Code extension = commands, diagnostics, workspace writes, webview hosting
React webview = graph, assurance, and artefact rendering
```

The extension does not derive protocol semantics from YAML text, generated
code, or graph layout. It calls the `aikenflow` CLI and renders the exported
bundle.

## Setup

1. Build or install the CLI:

   ```bash
   cargo build -p aikenflow-cli
   ```

2. Make `aikenflow` available on `PATH`, set `AIKENFLOW_BIN`, or configure:

   ```json
   {
     "aikenflow.backendPath": "/absolute/path/to/aikenflow"
   }
   ```

3. Open a workspace containing `protocol.yaml`, `protocol.yml`,
   `aikenflow.yaml`, or `aikenflow.yml`.

## Commands

```text
AikenFlow: Analyse Protocol
AikenFlow: Open Protocol Cockpit
AikenFlow: Generate Artefacts
AikenFlow: Export Audit Report
AikenFlow: Select Backend Binary
```

`AikenFlow: Analyse Protocol` runs `aikenflow check <spec> --json`, publishes
diagnostics to VS Code Problems, and exports `.aikenflow/bundle.json` when the
protocol is valid.

`AikenFlow: Open Protocol Cockpit` loads the Rust-exported bundle into the
React webview. The webview is bundle-driven only.

## Limits

AikenFlow v0.1 is not a formal verification tool. Assurance output is a review
surface generated from explicit protocol IR. Unsupported constraints and
partial invariant coverage require manual review.
