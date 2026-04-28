use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn aikenflow_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_aikenflow"))
}

fn run<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(aikenflow_bin())
        .args(args)
        .output()
        .expect("run aikenflow")
}

fn run_with_missing_ast_outline<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let missing_ast_outline = std::env::temp_dir().join(format!(
        "aikenflow-missing-ast-outline-{}",
        std::process::id()
    ));
    Command::new(aikenflow_bin())
        .args(args)
        .env("AIKENFLOW_AST_OUTLINE", missing_ast_outline)
        .output()
        .expect("run aikenflow")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn write(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write fixture");
}

#[test]
fn check_accepts_valid_protocol() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    write(&spec, SIMPLE_VAULT);

    let output = run(["check", spec.to_str().expect("utf-8 path")]);

    assert_success(&output);
    assert!(stdout(&output).contains("OK: `SimpleVault` has no diagnostics"));
}

#[test]
fn check_fails_on_semantic_errors() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    write(
        &spec,
        r#"
protocol: Broken
states:
  Locked:
    datum:
      owner: PubKeyHash
transitions:
  Spend:
    inputs:
      - state: Missing
    outputs: []
invariants:
  - name: reviewed
    expression: "manual review"
"#,
    );

    let output = run(["check", spec.to_str().expect("utf-8 path")]);

    assert_failure(&output);
    assert!(stdout(&output).contains("AF024"));
    assert!(stderr(&output).contains("protocol check failed"));
}

#[test]
fn check_json_uses_lowercase_diagnostic_severity() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    write(
        &spec,
        r#"
protocol: Broken
states:
  Locked:
    datum:
      owner: PubKeyHash
transitions:
  Spend:
    inputs:
      - state: Missing
    outputs: []
invariants:
  - name: reviewed
    expression: "manual review"
"#,
    );

    let output = run(["check", spec.to_str().expect("utf-8 path"), "--json"]);

    assert_failure(&output);
    let diagnostics: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("diagnostics json");
    assert_eq!(diagnostics["ok"], false);
    assert_eq!(diagnostics["diagnostics"][0]["severity"], "error");
    assert_eq!(
        diagnostics["diagnostics"][0]["target"]["line"],
        serde_json::Value::Null
    );
}

#[test]
fn check_rejects_duplicate_input_bindings() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    write(
        &spec,
        r#"
protocol: DuplicateInputs
states:
  Locked:
    datum:
      owner: PubKeyHash
transitions:
  SpendBoth:
    inputs:
      - state: Locked
      - state: Locked
    outputs: []
    constraints:
      - signed_by: "datum.owner"
invariants:
  - name: reviewed
    expression: "manual review"
"#,
    );

    let output = run(["check", spec.to_str().expect("utf-8 path")]);

    assert_failure(&output);
    assert!(stdout(&output).contains("AF046"));
    assert!(stdout(&output).contains("duplicate input binding"));
}

#[test]
fn check_rejects_undeclared_asset_references() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    write(
        &spec,
        r#"
protocol: BadAsset
assets:
  - name: ada
    kind: lovelace
states:
  Locked:
    datum:
      owner: PubKeyHash
transitions:
  Create:
    inputs: []
    outputs:
      - state: Locked
        value:
          missing_token: "1"
    constraints:
      - signed_by: "$owner"
invariants:
  - name: reviewed
    expression: "manual review"
"#,
    );

    let output = run(["check", spec.to_str().expect("utf-8 path")]);

    assert_failure(&output);
    assert!(stdout(&output).contains("AF051"));
    assert!(stdout(&output).contains("undeclared asset"));
}

#[test]
fn check_rejects_lovelace_mint_effects() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    write(
        &spec,
        r#"
protocol: BadMint
assets:
  - name: ada
    kind: lovelace
states:
  Locked:
    datum:
      owner: PubKeyHash
transitions:
  MintAda:
    inputs: []
    outputs: []
    mints:
      - policy: ada_policy
        asset: ada
        amount: "1"
    constraints:
      - signed_by: "$owner"
invariants:
  - name: reviewed
    expression: "manual review"
"#,
    );

    let output = run(["check", spec.to_str().expect("utf-8 path")]);

    assert_failure(&output);
    assert!(stdout(&output).contains("AF058"));
    assert!(stdout(&output).contains("lovelace asset"));
}

#[test]
fn check_rejects_codegen_unsafe_names() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    write(
        &spec,
        r#"
protocol: 123Bad
states:
  123Locked:
    datum:
      123owner: PubKeyHash
transitions:
  123Spend:
    inputs:
      - state: 123Locked
        alias: 123input
    outputs: []
invariants:
  - name: reviewed
    expression: "manual review"
"#,
    );

    let output = run(["check", spec.to_str().expect("utf-8 path")]);

    assert_failure(&output);
    for code in ["AF061", "AF062", "AF063", "AF064", "AF065"] {
        assert!(stdout(&output).contains(code), "missing {code}");
    }
}

#[test]
fn check_rejects_reserved_codegen_names() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    write(
        &spec,
        r#"
protocol: type
states:
  validator:
    datum:
      return: PubKeyHash
transitions:
  function:
    inputs:
      - state: validator
        alias: class
    outputs: []
invariants:
  - name: reviewed
    expression: "manual review"
"#,
    );

    let output = run(["check", spec.to_str().expect("utf-8 path")]);

    assert_failure(&output);
    for code in ["AF061", "AF062", "AF063", "AF064", "AF065"] {
        assert!(stdout(&output).contains(code), "missing {code}");
    }
}

#[test]
fn check_rejects_incompatible_datum_field_equals() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    write(
        &spec,
        r#"
protocol: BadDatumCheck
states:
  Locked:
    datum:
      owner: PubKeyHash
      amount: Int
transitions:
  Spend:
    inputs:
      - state: Locked
    outputs: []
    constraints:
      - datum_field_equals:
          field: datum.amount
          value: datum.owner
invariants:
  - name: reviewed
    expression: "manual review"
"#,
    );

    let output = run(["check", spec.to_str().expect("utf-8 path")]);

    assert_failure(&output);
    assert!(stdout(&output).contains("AF067"));
    assert!(stdout(&output).contains("incompatible type"));
}

#[test]
fn gen_all_writes_expected_artefacts() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    let out = temp.path().join("generated");
    write(&spec, SIMPLE_VAULT);

    let output = run([
        "gen",
        "all",
        spec.to_str().expect("utf-8 path"),
        "--out",
        out.to_str().expect("utf-8 path"),
    ]);

    assert_success(&output);
    assert!(out.join("contracts/aiken.toml").exists());
    assert!(out
        .join("contracts/validators/simple_vault_locked.ak")
        .exists());
    assert!(out.join("offchain/src/index.ts").exists());
    assert!(out.join("assurance/AUDIT.md").exists());
    assert!(out.join(".aikenflow-generated.json").exists());
}

#[test]
fn gen_removes_stale_manifest_files_without_deleting_user_files() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    let out = temp.path().join("generated");
    write(&spec, SIMPLE_VAULT);

    let gen_all = run([
        "gen",
        "all",
        spec.to_str().expect("utf-8 path"),
        "--out",
        out.to_str().expect("utf-8 path"),
    ]);
    assert_success(&gen_all);
    write(out.join("notes.md"), "user-owned notes\n");

    let gen_aiken = run([
        "gen",
        "aiken",
        spec.to_str().expect("utf-8 path"),
        "--out",
        out.to_str().expect("utf-8 path"),
    ]);
    assert_success(&gen_aiken);

    assert!(out.join("aiken.toml").exists());
    assert!(out.join("validators/simple_vault_locked.ak").exists());
    assert!(!out.join("contracts/aiken.toml").exists());
    assert!(!out.join("contracts").exists());
    assert!(!out.join("offchain/src/index.ts").exists());
    assert!(!out.join("offchain").exists());
    assert!(!out.join("assurance/AUDIT.md").exists());
    assert!(!out.join("assurance").exists());
    assert!(out.join("notes.md").exists());

    let manifest =
        fs::read_to_string(out.join(".aikenflow-generated.json")).expect("read manifest");
    assert!(manifest.contains("\"aiken.toml\""));
    assert!(!manifest.contains("offchain/src/index.ts"));
}

#[test]
fn gen_tests_writes_direct_test_files() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    let out = temp.path().join("tests");
    write(&spec, SIMPLE_VAULT);

    let output = run([
        "gen",
        "tests",
        spec.to_str().expect("utf-8 path"),
        "--out",
        out.to_str().expect("utf-8 path"),
    ]);

    assert_success(&output);
    assert!(out.join("adversarial.spec.ts").exists());
    assert!(!out.join("tests/adversarial.spec.ts").exists());

    let manifest =
        fs::read_to_string(out.join(".aikenflow-generated.json")).expect("read manifest");
    assert!(manifest.contains("\"adversarial.spec.ts\""));
    assert!(!manifest.contains("tests/adversarial.spec.ts"));
}

#[test]
fn export_writes_frontend_bundle() {
    let temp = TempDir::new().expect("temp dir");
    let spec = temp.path().join("protocol.yaml");
    let out = temp.path().join("bundle.json");
    write(&spec, SIMPLE_VAULT);

    let output = run([
        "export",
        spec.to_str().expect("utf-8 path"),
        "--out",
        out.to_str().expect("utf-8 path"),
    ]);

    assert_success(&output);
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out).expect("read bundle")).expect("bundle json");
    assert_eq!(bundle["schemaVersion"], "0.1.0");
    assert_eq!(bundle["protocol"]["name"], "SimpleVault");
    assert!(bundle["protocol"]["sourceYaml"]
        .as_str()
        .expect("source yaml")
        .contains("protocol: SimpleVault"));
    assert!(bundle["graph"]["states"].as_array().expect("states").len() == 1);
    assert!(!bundle["graph"]["transactions"]
        .as_array()
        .expect("transactions")
        .is_empty());
    assert!(!bundle["graph"]["edges"]
        .as_array()
        .expect("edges")
        .is_empty());
    assert!(!bundle["findings"].as_array().expect("findings").is_empty());
    assert!(bundle["artefacts"]["aiken"]
        .as_array()
        .expect("aiken artefacts")
        .iter()
        .any(|artefact| artefact["path"] == "contracts/validators/simple_vault_locked.ak"));
    assert!(bundle["artefacts"]["lucid"]
        .as_array()
        .expect("lucid artefacts")
        .iter()
        .any(|artefact| artefact["path"] == "offchain/src/index.ts"));
    assert!(
        bundle["metrics"]["generatedFiles"]
            .as_u64()
            .expect("generated files")
            > 0
    );
}

#[test]
fn draft_protocol_writes_review_only_report_without_ast_outline() {
    let temp = TempDir::new().expect("temp dir");
    write(
        temp.path().join("validators/vault.ak"),
        r#"
pub type VaultDatum {
  owner: ByteArray,
}

validator vault {
  spend(_datum, _redeemer, _ref, _tx) {
    True
  }
}
"#,
    );
    let out = temp.path().join("draft.md");

    let output = run([
        "draft-protocol",
        temp.path().to_str().expect("utf-8 path"),
        "--out",
        out.to_str().expect("utf-8 path"),
    ]);

    assert_success(&output);
    let draft = fs::read_to_string(out).expect("read draft");
    assert!(draft.contains("DRAFT ONLY"));
    assert!(draft.contains("not an AikenFlow protocol spec"));
    assert!(draft.contains("      owner: ByteArray"));
    assert!(draft.contains("Candidate from validator `vault`"));
}

#[test]
fn agent_context_reports_missing_ast_outline() {
    let temp = TempDir::new().expect("temp dir");

    let output = run_with_missing_ast_outline([
        "agent-context",
        temp.path().to_str().expect("utf-8 path"),
        "--for",
        "codex",
    ]);

    assert_failure(&output);
    assert!(stderr(&output).contains("ast-outline is not installed or not configured"));
    assert!(stderr(&output).contains("AIKENFLOW_AST_OUTLINE"));
}

#[test]
fn init_creates_template_that_can_be_checked() {
    let temp = TempDir::new().expect("temp dir");
    let out = temp.path().join("starter");

    let init = run([
        "init",
        "simple-vault",
        "--out",
        out.to_str().expect("utf-8 path"),
    ]);
    assert_success(&init);
    assert!(out.join("protocol.yaml").exists());
    assert!(out.join("README.md").exists());

    let check = run([
        "check",
        out.join("protocol.yaml").to_str().expect("utf-8 path"),
    ]);
    assert_success(&check);
}

#[test]
fn all_templates_can_check_and_generate() {
    let temp = TempDir::new().expect("temp dir");

    for template in ["simple-vault", "auction-lite", "programmable-token-mini"] {
        let out = temp.path().join(template);
        let generated = out.join("generated");

        let init = run(["init", template, "--out", out.to_str().expect("utf-8 path")]);
        assert_success(&init);

        let check = run([
            "check",
            out.join("protocol.yaml").to_str().expect("utf-8 path"),
        ]);
        assert_success(&check);

        let gen = run([
            "gen",
            "all",
            out.join("protocol.yaml").to_str().expect("utf-8 path"),
            "--out",
            generated.to_str().expect("utf-8 path"),
        ]);
        assert_success(&gen);

        assert!(generated.join("contracts/aiken.toml").exists());
        assert!(generated.join("offchain/src/index.ts").exists());
        assert!(generated.join("assurance/AUDIT.md").exists());
    }
}

const SIMPLE_VAULT: &str = r#"
protocol: SimpleVault
assets:
  - name: ada
    kind: lovelace
states:
  Locked:
    datum:
      owner: PubKeyHash
      amount: Lovelace
      deadline: POSIXTime
transitions:
  Deposit:
    inputs: []
    outputs:
      - state: Locked
        value:
          ada: "$amount"
    constraints:
      - signed_by: "$owner"
      - positive: "$amount"
  Withdraw:
    inputs:
      - state: Locked
    outputs: []
    constraints:
      - signed_by: "datum.owner"
      - after: "datum.deadline"
invariants:
  - name: withdrawal_requires_owner
    expression: "transition.Withdraw requires signature(datum.owner)"
"#;
