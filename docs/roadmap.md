# Roadmap

## Phase 1: MVP Compiler

- Stabilize YAML spec shape.
- Expand diagnostics for datum preservation and transition topology.
- Generate Aiken tests for supported constraints.
- Make generated Lucid Evolution integration concrete instead of structural.

## Phase 2: Assurance Expansion

- Generate negative tests with local fixtures.
- Add invariant coverage mapping rules.
- Add Yaci Dev Kit workflow.
- Add CI-friendly golden output tests.

## Phase 3: VS Code Protocol Cockpit

- Ship `extensions/vscode-aikenflow` as the production frontend.
- Run real `aikenflow export`, `check`, `gen`, `audit`, and `agent-context`
  commands from the extension host.
- Render the protocol graph, assurance panel, inspector, generated artefacts,
  and audit preview in a React webview.
- Publish protocol diagnostics into VS Code Problems and editor markers.

## Phase 4: Aiken Companion Integration

- Import Aiken blueprints.
- Generate builders and audit artefacts for existing Aiken projects.
- Add hook-based validator scaffolds.

## Phase 5: Additional Adapters

- Mesh transaction builder adapter.
- Blaze transaction builder adapter.
- cardano-cli script generation for reproducible demos.

## Phase 6: Advanced Protocols

- Multi-party auction.
- Programmable-token profile.
- Vault with multiple roles.
- DEX pool skeleton.
