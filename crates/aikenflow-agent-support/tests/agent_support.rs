use aikenflow_agent_support::{
    generate_agent_context_markdown, generate_protocol_draft_markdown, scan_repository,
    write_agent_context, write_protocol_draft, AgentSupportError, AstOutlineRunner,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn ast_outline_missing_is_reported_cleanly() {
    let temp = TempDir::new().expect("temp dir");
    let runner = AstOutlineRunner::new(temp.path().join("missing-ast-outline"));

    let error = generate_agent_context_markdown(temp.path(), &runner, "codex")
        .expect_err("missing ast-outline should fail");

    assert!(matches!(error, AgentSupportError::MissingAstOutline));
    assert!(error.to_string().contains("AIKENFLOW_AST_OUTLINE"));
}

#[test]
fn empty_project_generates_context_with_warnings() {
    let temp = TempDir::new().expect("temp dir");
    let tools = TempDir::new().expect("tool dir");
    let fake = fake_ast_outline(tools.path(), "");
    let runner = AstOutlineRunner::new(fake);

    let markdown =
        generate_agent_context_markdown(temp.path(), &runner, "codex").expect("context markdown");

    assert!(markdown.contains("# AikenFlow Agent Context"));
    assert!(markdown.contains("## Project Summary"));
    assert!(markdown.contains("No source files were found"));
    assert!(markdown.contains("ast-outline returned an empty outline"));
}

#[test]
fn sample_aiken_project_detects_validators_and_types() {
    let temp = TempDir::new().expect("temp dir");
    write(temp.path().join("protocol.yaml"), "protocol: Vault\n");
    write(temp.path().join("aiken.toml"), "name = \"vault\"\n");
    write(
        temp.path().join("validators/vault.ak"),
        r#"
use cardano/transaction.{OutputReference, Transaction}

pub type LockedDatum {
  owner: ByteArray,
}

pub type VaultRedeemer {
  Withdraw
}

validator vault {
  spend(datum: Option<LockedDatum>, redeemer: VaultRedeemer, _ref: OutputReference, self: Transaction) {
    True
  }
}

test withdraw_requires_owner() {
  True
}
"#,
    );
    write(temp.path().join("plutus.json"), "{}");
    let tools = TempDir::new().expect("tool dir");
    let fake = fake_ast_outline(tools.path(), "# validators/vault.ak (20 lines)\n");
    let runner = AstOutlineRunner::new(fake);

    let scan = scan_repository(temp.path()).expect("scan");
    assert_eq!(scan.aiken_files, vec![PathBuf::from("validators/vault.ak")]);
    assert_eq!(scan.validators[0].name, "vault");
    assert_eq!(scan.datum_redeemer_types.len(), 2);
    assert_eq!(scan.datum_redeemer_types[0].fields[0].name, "owner");
    assert_eq!(scan.datum_redeemer_types[0].fields[0].ty, "ByteArray");
    assert_eq!(scan.tests[0].name, "withdraw_requires_owner");
    assert_eq!(scan.blueprints, vec![PathBuf::from("plutus.json")]);
    assert_eq!(scan.protocol_specs, vec![PathBuf::from("protocol.yaml")]);
    assert_eq!(scan.aiken_manifests, vec![PathBuf::from("aiken.toml")]);

    let markdown =
        generate_agent_context_markdown(temp.path(), &runner, "codex").expect("context markdown");
    assert!(markdown.contains("## Protocol Specs"));
    assert!(markdown.contains("## Aiken Manifests"));
    assert!(markdown.contains("`validator` `validator vault {`"));
    assert!(markdown.contains("`type` `pub type LockedDatum {`"));
    assert!(markdown.contains("`plutus.json`"));
}

#[test]
fn mixed_aiken_and_typescript_project_detects_builders() {
    let temp = TempDir::new().expect("temp dir");
    write(
        temp.path().join("validators/order.ak"),
        "pub type OrderDatum { owner: ByteArray }\nvalidator order { }\n",
    );
    write(
        temp.path().join("offchain/src/order.ts"),
        r#"
import { Lucid } from "@lucid-evolution/lucid";

export async function fillOrder(lucid: Lucid, utxo: UTxO) {
  return lucid.newTx().collectFrom([utxo]).payToContract("addr", {}, {}).complete();
}
"#,
    );
    let tools = TempDir::new().expect("tool dir");
    let fake = fake_ast_outline(
        tools.path(),
        "# offchain/src/order.ts (6 lines)\nexport async function fillOrder(lucid: Lucid, utxo: UTxO)  L4-6\n",
    );
    let runner = AstOutlineRunner::new(fake);

    let markdown =
        generate_agent_context_markdown(temp.path(), &runner, "codex").expect("context markdown");

    assert!(markdown.contains("## Off-chain Transaction Builders"));
    assert!(markdown.contains("offchain/src/order.ts"));
    assert!(markdown.contains("newTx"));
    assert!(markdown.contains("collectFrom"));
}

#[test]
fn agent_context_normalizes_ast_outline_paths_to_target_root() {
    let temp = TempDir::new().expect("temp dir");
    write(
        temp.path().join("src/index.ts"),
        "export function buildTx() { return lucid.newTx(); }\n",
    );
    let tools = TempDir::new().expect("tool dir");
    let raw = format!(
        "# {} (1 lines)\nexport function buildTx()  L1\n",
        temp.path().join("src/index.ts").display()
    );
    let fake = fake_ast_outline(tools.path(), &raw);
    let runner = AstOutlineRunner::new(fake);

    let markdown =
        generate_agent_context_markdown(temp.path(), &runner, "codex").expect("context markdown");
    let outline_section = markdown
        .split("## AST Outline Summary")
        .nth(1)
        .expect("outline section");

    assert!(markdown.contains("- `src/index.ts`"));
    assert!(!outline_section.contains(temp.path().to_string_lossy().as_ref()));
}

#[test]
fn generated_mermaid_files_are_not_reported_as_unsupported() {
    let temp = TempDir::new().expect("temp dir");
    write(
        temp.path().join("generated/assurance/state-graph.mmd"),
        "stateDiagram-v2\n",
    );
    let tools = TempDir::new().expect("tool dir");
    let fake = fake_ast_outline(tools.path(), "");
    let runner = AstOutlineRunner::new(fake);

    let markdown =
        generate_agent_context_markdown(temp.path(), &runner, "codex").expect("context markdown");

    assert!(markdown.contains("generated/assurance/state-graph.mmd"));
    assert!(!markdown.contains("unsupported `.mmd`"));
}

#[test]
fn test_files_need_builder_markers_to_be_transaction_builders() {
    let temp = TempDir::new().expect("temp dir");
    write(
        temp.path().join("tests/adversarial.spec.ts"),
        "import { test } from \"node:test\";\n",
    );
    write(
        temp.path().join("src/build.ts"),
        "export function build(lucid) { return lucid.newTx().collectFrom([]); }\n",
    );
    let scan = scan_repository(temp.path()).expect("scan");

    let test_file = scan
        .offchain_files
        .iter()
        .find(|file| file.path == Path::new("tests/adversarial.spec.ts"))
        .expect("test file");
    let build_file = scan
        .offchain_files
        .iter()
        .find(|file| file.path == Path::new("src/build.ts"))
        .expect("build file");

    assert!(!test_file.likely_transaction_builder);
    assert!(build_file.likely_transaction_builder);
}

#[test]
fn type_support_files_are_not_transaction_builders() {
    let temp = TempDir::new().expect("temp dir");
    write(
        temp.path().join("src/types.ts"),
        r#"
export interface TxBuilder {
  newTx(): TxBuilder;
  collectFrom(utxos: unknown[]): TxBuilder;
  payToContract(address: string, datum: unknown, assets: unknown): TxBuilder;
}
"#,
    );
    write(
        temp.path().join("src/build.ts"),
        "export function build(lucid) { return lucid.newTx().collectFrom([]); }\n",
    );

    let scan = scan_repository(temp.path()).expect("scan");
    let types_file = scan
        .offchain_files
        .iter()
        .find(|file| file.path == Path::new("src/types.ts"))
        .expect("types file");
    let build_file = scan
        .offchain_files
        .iter()
        .find(|file| file.path == Path::new("src/build.ts"))
        .expect("build file");

    assert!(!types_file.markers.is_empty());
    assert!(!types_file.likely_transaction_builder);
    assert!(build_file.likely_transaction_builder);
}

#[test]
fn writes_agent_context_to_aikenflow_directory() {
    let temp = TempDir::new().expect("temp dir");
    write(
        temp.path().join("validators/vault.ak"),
        "validator vault { }\n",
    );
    let tools = TempDir::new().expect("tool dir");
    let fake = fake_ast_outline(tools.path(), "# validators/vault.ak (1 lines)\n");
    let runner = AstOutlineRunner::new(fake);

    let out = write_agent_context(temp.path(), &runner, "codex").expect("write context");
    let content = fs::read_to_string(&out).expect("read context");

    assert_eq!(out, temp.path().join(".aikenflow/agent-context.md"));
    assert!(content.contains("# AikenFlow Agent Context"));
    assert!(content.contains("## Validators"));
    assert!(content.contains("## Suggested Next Files To Inspect"));
    assert!(content.contains("## Suggested Next Edit Points"));
    assert!(content.contains("validators/vault.ak:1"));
    assert!(content.contains("Boundary: `ast-outline`"));
}

#[test]
fn protocol_draft_is_labeled_and_uses_candidates() {
    let temp = TempDir::new().expect("temp dir");
    write(
        temp.path().join("validators/order.ak"),
        r#"
pub type OrderDatum {
  owner: ByteArray,
}

pub type OrderRedeemer {
  Fill
}

validator order {
  spend(_datum, _redeemer, _ref, _tx) {
    True
  }
}
"#,
    );
    write(
        temp.path().join("offchain/fill.ts"),
        "export function fill(lucid, utxo) { return lucid.newTx().collectFrom([utxo]); }\n",
    );

    let markdown = generate_protocol_draft_markdown(temp.path()).expect("protocol draft");

    assert!(markdown.contains("# AikenFlow Existing Project Protocol Draft"));
    assert!(markdown.contains("THIS IS A DRAFT. NOT VERIFIED."));
    assert!(markdown.contains("DRAFT ONLY"));
    assert!(markdown.contains("not an AikenFlow protocol spec"));
    assert!(markdown.contains("## Candidate Validators"));
    assert!(markdown.contains("`validator` `validator order {`"));
    assert!(markdown.contains("## Draft Protocol Sketch"));
    assert!(markdown.contains("  Order:"));
    assert!(markdown.contains("      owner: ByteArray"));
    assert!(markdown.contains("Candidate from validator `order`"));
    assert!(markdown.contains("offchain/fill.ts"));
    assert!(markdown.contains("Required Review Before Compiler Use"));
}

#[test]
fn writes_protocol_draft_to_aikenflow_directory() {
    let temp = TempDir::new().expect("temp dir");
    write(
        temp.path().join("validators/vault.ak"),
        "pub type VaultDatum { owner: ByteArray }\nvalidator vault { }\n",
    );

    let out = write_protocol_draft(temp.path(), None).expect("write protocol draft");
    let content = fs::read_to_string(&out).expect("read protocol draft");

    assert_eq!(out, temp.path().join(".aikenflow/protocol-draft.md"));
    assert!(content.contains("# AikenFlow Existing Project Protocol Draft"));
    assert!(content.contains("THIS IS A DRAFT. NOT VERIFIED."));
    assert!(content.contains("DRAFT ONLY"));
    assert!(content.contains("Vault"));
}

fn write(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write fixture");
}

fn fake_ast_outline(root: &Path, stdout: &str) -> PathBuf {
    let script = root.join("fake-ast-outline.sh");
    let escaped = stdout.replace('\\', "\\\\").replace('\'', "'\"'\"'");
    fs::write(&script, format!("#!/bin/sh\nprintf '%s' '{}'\n", escaped))
        .expect("write fake ast-outline");

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&script).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).expect("chmod");
    }

    script
}
