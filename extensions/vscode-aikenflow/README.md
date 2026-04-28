# AikenFlow for VS Code

AikenFlow for VS Code embeds the protocol cockpit into an existing editor
workspace. The extension is a thin client over the Rust `aikenflow` CLI.

## Required CLI

Install the CLI from a GitHub Release, or build it locally from the repository
root and make it available on `PATH`:

```bash
cargo install --path crates/aikenflow-cli
```

For local development, set:

```json
{
  "aikenflow.backendPath": "/absolute/path/to/AikenFlow/target/debug/aikenflow"
}
```

## Commands

- `AikenFlow: Analyse Protocol`
- `AikenFlow: Open Protocol Cockpit`
- `AikenFlow: Generate Artefacts`
- `AikenFlow: Export Audit Report`
- `AikenFlow: Create Agent Context`
- `AikenFlow: Select Backend Binary`

All production command paths call the Rust CLI. The webview renders exported
bundles and never derives protocol semantics by itself.

## Verification

```bash
npm audit --audit-level=moderate
npm run check
npm run check:full
```

`check:full` builds the webview, runs unit tests, launches VS Code through
`@vscode/test-electron` against a temporary fixture workspace, and packages a
VSIX.

## Release

The extension is packaged as `vscode-aikenflow-<version>.vsix`. Marketplace
publishing is handled by the repository release workflow when `VSCE_PAT` is
configured for the `tsukifune-kosei` publisher.
