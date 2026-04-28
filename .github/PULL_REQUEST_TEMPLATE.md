## Summary

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace --no-fail-fast`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo audit`
- [ ] `cd extensions/vscode-aikenflow && npm audit --audit-level=moderate`
- [ ] `cd extensions/vscode-aikenflow && npm run check:full`
- [ ] `bash scripts/verify-examples.sh`

## Boundary Check

- [ ] Rust remains the semantic authority.
- [ ] VS Code extension only orchestrates CLI commands and editor integration.
- [ ] Webview renders exported bundle data only.
- [ ] Agent-support heuristics are not used as compiler semantics.
