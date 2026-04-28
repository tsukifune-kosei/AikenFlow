use aikenflow_assurance as assurance;
use aikenflow_ir::{Diagnostic, DiagnosticSeverity, GeneratedFiles, Protocol};
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const GENERATED_MANIFEST: &str = ".aikenflow-generated.json";
const GENERATED_MANIFEST_VERSION: u8 = 2;

#[derive(Debug, Parser)]
#[command(name = "aikenflow")]
#[command(about = "Aiken-compatible eUTxO protocol compiler and assurance toolkit")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(value_enum)]
        template: Template,
        #[arg(long, short)]
        out: Option<PathBuf>,
    },
    Check {
        spec: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Graph {
        spec: PathBuf,
        #[arg(long, value_enum, default_value_t = GraphFormat::Mermaid)]
        format: GraphFormat,
        #[arg(long, short)]
        out: Option<PathBuf>,
    },
    Gen {
        #[arg(value_enum)]
        target: GenTarget,
        spec: PathBuf,
        #[arg(long, short, default_value = "generated")]
        out: PathBuf,
    },
    Audit {
        spec: PathBuf,
        #[arg(long, short, default_value = "AUDIT.md")]
        out: PathBuf,
    },
    Export {
        spec: PathBuf,
        #[arg(long, short)]
        out: PathBuf,
    },
    Outline {
        path: PathBuf,
    },
    AgentContext {
        path: PathBuf,
        #[arg(long = "for", value_enum)]
        agent: AgentTarget,
    },
    DraftProtocol {
        path: PathBuf,
        #[arg(long, short)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Template {
    SimpleVault,
    AuctionLite,
    ProgrammableTokenMini,
}

impl Template {
    fn directory_name(self) -> &'static str {
        match self {
            Template::SimpleVault => "simple-vault",
            Template::AuctionLite => "auction-lite",
            Template::ProgrammableTokenMini => "programmable-token-mini",
        }
    }

    fn protocol_yaml(self) -> &'static str {
        match self {
            Template::SimpleVault => SIMPLE_VAULT,
            Template::AuctionLite => AUCTION_LITE,
            Template::ProgrammableTokenMini => PROGRAMMABLE_TOKEN_MINI,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum GenTarget {
    All,
    Aiken,
    Lucid,
    Assurance,
    Tests,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum GraphFormat {
    Mermaid,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AgentTarget {
    Codex,
}

impl AgentTarget {
    fn as_str(self) -> &'static str {
        match self {
            AgentTarget::Codex => "codex",
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { template, out } => init(template, out),
        Command::Check { spec, json } => check(&spec, json),
        Command::Graph { spec, format, out } => graph(&spec, format, out),
        Command::Gen { target, spec, out } => gen(target, &spec, &out),
        Command::Audit { spec, out } => audit(&spec, &out),
        Command::Export { spec, out } => export(&spec, &out),
        Command::Outline { path } => outline(&path),
        Command::AgentContext { path, agent } => agent_context(&path, agent),
        Command::DraftProtocol { path, out } => draft_protocol(&path, out.as_deref()),
    }
}

fn init(template: Template, out: Option<PathBuf>) -> Result<()> {
    let out = out.unwrap_or_else(|| PathBuf::from(template.directory_name()));
    fs::create_dir_all(&out).with_context(|| format!("failed to create `{}`", out.display()))?;
    fs::write(out.join("protocol.yaml"), template.protocol_yaml())
        .with_context(|| format!("failed to write `{}`", out.join("protocol.yaml").display()))?;
    fs::write(out.join("README.md"), template_readme(template))
        .with_context(|| format!("failed to write `{}`", out.join("README.md").display()))?;

    println!("Created {}", out.display());
    println!("Next:");
    println!("  cd {}", out.display());
    println!("  aikenflow check protocol.yaml");
    println!("  aikenflow gen all protocol.yaml --out generated");
    Ok(())
}

fn check(spec: &Path, json: bool) -> Result<()> {
    let protocol = load_protocol(spec)?;
    let diagnostics = protocol.validate();
    let ok = !diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&CheckJsonOutput::new(ok, &diagnostics))?
        );
    } else if diagnostics.is_empty() {
        println!("OK: `{}` has no diagnostics", protocol.name);
    } else {
        print_diagnostics(&diagnostics);
    }

    if !ok {
        bail!("protocol check failed");
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct CheckJsonOutput {
    ok: bool,
    diagnostics: Vec<CheckJsonDiagnostic>,
}

impl CheckJsonOutput {
    fn new(ok: bool, diagnostics: &[Diagnostic]) -> Self {
        Self {
            ok,
            diagnostics: diagnostics.iter().map(CheckJsonDiagnostic::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct CheckJsonDiagnostic {
    severity: DiagnosticSeverity,
    code: String,
    message: String,
    hint: String,
    target: CheckJsonDiagnosticTarget,
}

impl From<&Diagnostic> for CheckJsonDiagnostic {
    fn from(diagnostic: &Diagnostic) -> Self {
        Self {
            severity: diagnostic.severity,
            code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
            hint: diagnostic.hint.clone(),
            target: CheckJsonDiagnosticTarget::default(),
        }
    }
}

#[derive(Debug, Default, Serialize)]
struct CheckJsonDiagnosticTarget {
    kind: Option<String>,
    name: Option<String>,
    field: Option<String>,
    line: Option<usize>,
    column: Option<usize>,
}

fn graph(spec: &Path, format: GraphFormat, out: Option<PathBuf>) -> Result<()> {
    let protocol = load_protocol(spec)?;
    fail_on_validation_errors(&protocol)?;

    let content = match format {
        GraphFormat::Mermaid => assurance::generate_state_graph(&protocol),
        GraphFormat::Json => serde_json::to_string_pretty(&protocol.topology())?,
    };

    if let Some(path) = out {
        write_file(&path, &content)?;
    } else {
        println!("{content}");
    }

    Ok(())
}

fn gen(target: GenTarget, spec: &Path, out: &Path) -> Result<()> {
    let protocol = load_protocol(spec)?;
    fail_on_validation_errors(&protocol)?;

    let mut files = GeneratedFiles::default();
    match target {
        GenTarget::All => {
            files.extend(prefix_files(
                "contracts",
                aikenflow_aiken_gen::generate(&protocol),
            ));
            files.extend(prefix_files(
                "offchain",
                aikenflow_lucid_gen::generate(&protocol),
            ));
            files.extend(prefix_files("assurance", assurance::generate(&protocol)));
        }
        GenTarget::Aiken => files.extend(aikenflow_aiken_gen::generate(&protocol)),
        GenTarget::Lucid => files.extend(aikenflow_lucid_gen::generate(&protocol)),
        GenTarget::Assurance => files.extend(assurance::generate(&protocol)),
        GenTarget::Tests => {
            let mut assurance_files = assurance::generate(&protocol);
            assurance_files
                .files
                .retain(|file| file.path.starts_with("tests/"));
            strip_file_prefix("tests", &mut assurance_files);
            files.extend(assurance_files);
        }
    }

    write_generated_files(out, files)?;
    println!(
        "Generated {} artefact set into {}",
        target_name(target),
        out.display()
    );
    Ok(())
}

fn audit(spec: &Path, out: &Path) -> Result<()> {
    let protocol = load_protocol(spec)?;
    fail_on_validation_errors(&protocol)?;
    write_file(out, &assurance::generate_audit_report(&protocol))?;
    println!("Wrote {}", out.display());
    Ok(())
}

fn export(spec: &Path, out: &Path) -> Result<()> {
    let source =
        fs::read_to_string(spec).with_context(|| format!("failed to read `{}`", spec.display()))?;
    let protocol = aikenflow_parser::parse_protocol(&source)
        .with_context(|| format!("failed to parse `{}`", spec.display()))?;
    fail_on_validation_errors(&protocol)?;

    let bundle = aikenflow_export::export_bundle(&protocol, source);
    let json = format!("{}\n", serde_json::to_string_pretty(&bundle)?);
    write_file(out, &json)?;
    println!("Wrote {}", out.display());
    Ok(())
}

fn outline(path: &Path) -> Result<()> {
    let runner = aikenflow_agent_support::AstOutlineRunner::from_env();
    let markdown = aikenflow_agent_support::generate_outline_markdown(path, &runner)?;
    println!("{markdown}");
    Ok(())
}

fn agent_context(path: &Path, agent: AgentTarget) -> Result<()> {
    let runner = aikenflow_agent_support::AstOutlineRunner::from_env();
    let out = aikenflow_agent_support::write_agent_context(path, &runner, agent.as_str())?;
    println!("Wrote {}", out.display());
    Ok(())
}

fn draft_protocol(path: &Path, out: Option<&Path>) -> Result<()> {
    let out = aikenflow_agent_support::write_protocol_draft(path, out)?;
    println!("Wrote {}", out.display());
    println!("Draft only: review into a user-owned protocol.yaml before running check/gen/audit.");
    Ok(())
}

fn load_protocol(spec: &Path) -> Result<Protocol> {
    aikenflow_parser::parse_protocol_file(spec)
        .with_context(|| format!("failed to parse `{}`", spec.display()))
}

fn fail_on_validation_errors(protocol: &Protocol) -> Result<()> {
    let diagnostics = protocol.validate();
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        print_diagnostics(&diagnostics);
        bail!("protocol contains validation errors");
    }

    Ok(())
}

fn print_diagnostics(diagnostics: &[aikenflow_ir::Diagnostic]) {
    for diagnostic in diagnostics {
        println!(
            "{}[{}]: {}\n  hint: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message, diagnostic.hint
        );
    }
}

fn write_generated_files(root: &Path, files: GeneratedFiles) -> Result<()> {
    let generated_entries = generated_manifest_entries(&files)?;
    let generated_paths = generated_entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();

    reject_modified_generated_files(root, &generated_paths)?;
    cleanup_stale_generated_files(root, &generated_paths)?;

    for file in files.files {
        let path = validate_generated_relative_path(&file.path)?;
        write_file(&root.join(path), &file.content)?;
    }

    write_generation_manifest(root, &generated_entries)?;
    Ok(())
}

fn generated_manifest_entries(files: &GeneratedFiles) -> Result<Vec<GeneratedManifestEntry>> {
    let mut generated_paths = BTreeSet::new();
    let mut entries = Vec::new();

    for file in &files.files {
        let path = generated_manifest_path(&file.path)?;
        if !generated_paths.insert(path.clone()) {
            bail!("generated file path `{path}` is produced more than once");
        }
        entries.push(GeneratedManifestEntry {
            path,
            content_hash: content_hash(&file.content),
        });
    }

    Ok(entries)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedManifestEntry {
    path: String,
    content_hash: String,
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create `{}`", parent.display()))?;
        }
    }
    fs::write(path, content).with_context(|| format!("failed to write `{}`", path.display()))
}

fn cleanup_stale_generated_files(root: &Path, current: &BTreeSet<String>) -> Result<()> {
    let manifest = root.join(GENERATED_MANIFEST);
    if !manifest.exists() {
        return Ok(());
    }

    let previous = read_generation_manifest(&manifest)?;
    let previous_paths = previous
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();
    for relative in previous_paths.difference(current) {
        let path = validate_generated_relative_path(relative)?;
        let full_path = root.join(&path);
        let metadata = match fs::symlink_metadata(&full_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to inspect `{}`", full_path.display()));
            }
        };

        if metadata.is_file() || metadata.file_type().is_symlink() {
            fs::remove_file(&full_path)
                .with_context(|| format!("failed to remove stale `{}`", full_path.display()))?;
        }
    }

    prune_empty_generated_dirs(root, &previous_paths)?;
    Ok(())
}

fn reject_modified_generated_files(root: &Path, current: &BTreeSet<String>) -> Result<()> {
    let manifest = root.join(GENERATED_MANIFEST);
    if !manifest.exists() {
        reject_unmanaged_existing_targets(root, current)?;
        return Ok(());
    }

    let previous = read_generation_manifest(&manifest)?;
    let previous_paths = previous
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();

    for relative in current {
        let path = validate_generated_relative_path(relative)?;
        let full_path = root.join(&path);
        if !full_path.exists() {
            continue;
        }

        let Some(entry) = previous.iter().find(|entry| entry.path == *relative) else {
            bail!(
                "refusing to overwrite `{}` because it is not tracked by `{}`",
                full_path.display(),
                GENERATED_MANIFEST
            );
        };

        if entry.content_hash.is_empty() {
            continue;
        }

        let current_content = fs::read_to_string(&full_path)
            .with_context(|| format!("failed to read `{}`", full_path.display()))?;
        let current_hash = content_hash(&current_content);
        if current_hash != entry.content_hash {
            bail!(
                "refusing to overwrite modified generated file `{}`; move it aside or regenerate into a clean output directory",
                full_path.display()
            );
        }
    }

    for relative in previous_paths.difference(current) {
        let path = validate_generated_relative_path(relative)?;
        let full_path = root.join(&path);
        if !full_path.exists() {
            continue;
        }
        let Some(entry) = previous.iter().find(|entry| entry.path == *relative) else {
            continue;
        };
        if entry.content_hash.is_empty() {
            continue;
        }
        let current_content = fs::read_to_string(&full_path)
            .with_context(|| format!("failed to read `{}`", full_path.display()))?;
        if content_hash(&current_content) != entry.content_hash {
            bail!(
                "refusing to remove modified stale generated file `{}`; move it aside before regenerating",
                full_path.display()
            );
        }
    }

    Ok(())
}

fn reject_unmanaged_existing_targets(root: &Path, current: &BTreeSet<String>) -> Result<()> {
    for relative in current {
        let path = validate_generated_relative_path(relative)?;
        let full_path = root.join(&path);
        if full_path.exists() {
            bail!(
                "refusing to overwrite existing untracked file `{}`; choose a clean output directory or remove the file",
                full_path.display()
            );
        }
    }
    Ok(())
}

fn read_generation_manifest(path: &Path) -> Result<Vec<GeneratedManifestEntry>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse `{}`", path.display()))?;
    let files = value
        .get("files")
        .and_then(serde_json::Value::as_array)
        .context("generated manifest is missing a `files` array")?;

    let mut entries = Vec::new();
    let mut seen_paths = BTreeSet::new();
    for file in files {
        let (path, content_hash) = if let Some(path) = file.as_str() {
            (path.to_owned(), String::new())
        } else if let Some(object) = file.as_object() {
            let path = object
                .get("path")
                .and_then(serde_json::Value::as_str)
                .context("generated manifest file entries must include a string `path`")?
                .to_owned();
            let content_hash = object
                .get("contentHash")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            (path, content_hash)
        } else {
            bail!("generated manifest `files` entries must be strings or objects");
        };
        generated_manifest_path(&path)?;
        if seen_paths.insert(path.clone()) {
            entries.push(GeneratedManifestEntry { path, content_hash });
        }
    }

    Ok(entries)
}

fn write_generation_manifest(
    root: &Path,
    generated_entries: &[GeneratedManifestEntry],
) -> Result<()> {
    let files = generated_entries
        .iter()
        .map(|entry| {
            serde_json::json!({
                "path": entry.path,
                "contentHash": entry.content_hash,
            })
        })
        .collect::<Vec<_>>();
    let manifest = serde_json::json!({
        "version": GENERATED_MANIFEST_VERSION,
        "tool": "aikenflow",
        "files": files,
    });
    let content = format!("{}\n", serde_json::to_string_pretty(&manifest)?);
    write_file(&root.join(GENERATED_MANIFEST), &content)
}

fn content_hash(content: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn prune_empty_generated_dirs(root: &Path, previous: &BTreeSet<String>) -> Result<()> {
    let mut dirs = BTreeSet::new();
    for relative in previous {
        let Ok(path) = validate_generated_relative_path(relative) else {
            continue;
        };

        let mut parent = path.parent();
        while let Some(dir) = parent {
            if dir.as_os_str().is_empty() {
                break;
            }
            dirs.insert(dir.to_path_buf());
            parent = dir.parent();
        }
    }

    let mut dirs = dirs.into_iter().collect::<Vec<_>>();
    dirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));

    for dir in dirs {
        let full_path = root.join(&dir);
        if fs::read_dir(&full_path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
        {
            fs::remove_dir(&full_path)
                .with_context(|| format!("failed to remove empty `{}`", full_path.display()))?;
        }
    }

    Ok(())
}

fn generated_manifest_path(path: &str) -> Result<String> {
    let path = validate_generated_relative_path(path)?;
    Ok(path.to_string_lossy().replace('\\', "/"))
}

fn validate_generated_relative_path(path: &str) -> Result<PathBuf> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() {
        bail!("generated file path cannot be empty");
    }
    if path.is_absolute() {
        bail!("generated file path `{}` must be relative", path.display());
    }

    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => bail!(
                "generated file path `{}` contains an unsafe component",
                path.display()
            ),
        }
    }

    Ok(path.to_path_buf())
}

fn prefix_files(prefix: &str, mut files: GeneratedFiles) -> GeneratedFiles {
    for file in &mut files.files {
        file.path = format!("{prefix}/{}", file.path);
    }
    files
}

fn strip_file_prefix(prefix: &str, files: &mut GeneratedFiles) {
    let prefix = format!("{prefix}/");
    for file in &mut files.files {
        if let Some(stripped) = file.path.strip_prefix(&prefix) {
            file.path = stripped.to_owned();
        }
    }
}

fn target_name(target: GenTarget) -> &'static str {
    match target {
        GenTarget::All => "all",
        GenTarget::Aiken => "aiken",
        GenTarget::Lucid => "lucid",
        GenTarget::Assurance => "assurance",
        GenTarget::Tests => "tests",
    }
}

fn template_readme(template: Template) -> String {
    format!(
        "# {}\n\nGenerated AikenFlow starter protocol.\n\n```bash\naikenflow check protocol.yaml\naikenflow graph protocol.yaml\naikenflow gen all protocol.yaml --out generated\n```\n",
        template.directory_name()
    )
}

const SIMPLE_VAULT: &str = r#"protocol: SimpleVault
description: Simple custody workflow with owner authorization and deadline-based withdrawal.

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

  EmergencyCancel:
    inputs:
      - state: Locked
    outputs: []
    constraints:
      - signed_by: "datum.owner"
      - before: "datum.deadline"

invariants:
  - name: no_negative_locked_value
    expression: "locked.amount >= 0"
  - name: withdrawal_requires_owner
    expression: "transition.Withdraw requires signature(datum.owner)"
  - name: withdrawal_requires_deadline_passed
    expression: "transition.Withdraw requires after(datum.deadline)"
"#;

const AUCTION_LITE: &str = r#"protocol: AuctionLite
description: Minimal auction state machine for AikenFlow protocol compilation.

assets:
  - name: ada
    kind: lovelace

states:
  Open:
    datum:
      seller: PubKeyHash
      highestBidder: PubKeyHash
      highestBid: Lovelace
      deadline: POSIXTime
  Closed:
    datum:
      seller: PubKeyHash
      winner: PubKeyHash
      finalBid: Lovelace

transitions:
  CreateAuction:
    inputs: []
    outputs:
      - state: Open
        value:
          ada: "$minBid"
    constraints:
      - signed_by: "$seller"
      - positive: "$minBid"

  Bid:
    inputs:
      - state: Open
    outputs:
      - state: Open
        value:
          ada: "$bid"
    constraints:
      - signed_by: "$bidder"
      - before: "datum.deadline"
      - positive: "$bid"

  Close:
    inputs:
      - state: Open
    outputs:
      - state: Closed
        value:
          ada: "datum.highestBid"
    constraints:
      - signed_by: "datum.seller"
      - after: "datum.deadline"

invariants:
  - name: highest_bid_monotonic
    expression: "transition.Bid increases datum.highestBid"
  - name: close_requires_deadline
    expression: "transition.Close requires after(datum.deadline)"
"#;

const PROGRAMMABLE_TOKEN_MINI: &str = r#"protocol: ProgrammableTokenMini
description: Small programmable-token lifecycle profile inspired by freeze and revoke workflows.

assets:
  - name: regulated_token
    kind: native
    policy: regulated_policy

states:
  Active:
    datum:
      authority: PubKeyHash
      holder: PubKeyHash
      amount: Int
  Frozen:
    datum:
      authority: PubKeyHash
      holder: PubKeyHash
      amount: Int
  Revoked:
    terminal: true
    datum:
      authority: PubKeyHash
      amount: Int

transitions:
  Transfer:
    inputs:
      - state: Active
    outputs:
      - state: Active
    constraints:
      - signed_by: "datum.holder"
      - positive: "datum.amount"

  Freeze:
    inputs:
      - state: Active
    outputs:
      - state: Frozen
    constraints:
      - signed_by: "datum.authority"

  Unfreeze:
    inputs:
      - state: Frozen
    outputs:
      - state: Active
    constraints:
      - signed_by: "datum.authority"

  Revoke:
    inputs:
      - state: Frozen
    outputs:
      - state: Revoked
    constraints:
      - signed_by: "datum.authority"

invariants:
  - name: frozen_asset_cannot_transfer
    expression: "transition.Transfer only consumes Active"
  - name: authority_controls_freeze
    expression: "transition.Freeze requires signature(datum.authority)"
  - name: revoke_requires_frozen_state
    expression: "transition.Revoke consumes Frozen"
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use aikenflow_ir::GeneratedFiles;
    use tempfile::TempDir;

    #[test]
    fn bundled_templates_parse() {
        for template in [
            Template::SimpleVault,
            Template::AuctionLite,
            Template::ProgrammableTokenMini,
        ] {
            let protocol = aikenflow_parser::parse_protocol(template.protocol_yaml())
                .expect("template parses");
            assert!(
                !protocol.has_validation_errors(),
                "{} should have no validation errors",
                protocol.name
            );
        }
    }

    #[test]
    fn generated_manifest_rejects_duplicate_paths() {
        let mut files = GeneratedFiles::default();
        files.push("README.md", "first");
        files.push("README.md", "second");

        let error = generated_manifest_entries(&files).expect_err("duplicate path should fail");

        assert!(error.to_string().contains("produced more than once"));
    }

    #[test]
    fn write_generated_files_rejects_unsafe_paths() {
        let temp = TempDir::new().expect("temp dir");
        let mut files = GeneratedFiles::default();
        files.push("../escape.txt", "escape");

        let error = write_generated_files(temp.path(), files).expect_err("unsafe path should fail");

        assert!(error.to_string().contains("unsafe component"));
        assert!(!temp.path().join("../escape.txt").exists());
    }

    #[test]
    fn write_generated_files_rejects_modified_tracked_files() {
        let temp = TempDir::new().expect("temp dir");
        let mut first = GeneratedFiles::default();
        first.push("README.md", "generated\n");
        write_generated_files(temp.path(), first).expect("first generation");

        fs::write(temp.path().join("README.md"), "manual edit\n").expect("manual edit");

        let mut second = GeneratedFiles::default();
        second.push("README.md", "next generated\n");
        let error =
            write_generated_files(temp.path(), second).expect_err("manual edit should fail");

        assert!(error
            .to_string()
            .contains("refusing to overwrite modified generated file"));
    }

    #[test]
    fn write_generated_files_rejects_untracked_existing_targets() {
        let temp = TempDir::new().expect("temp dir");
        fs::write(temp.path().join("README.md"), "user file\n").expect("user file");

        let mut files = GeneratedFiles::default();
        files.push("README.md", "generated\n");
        let error =
            write_generated_files(temp.path(), files).expect_err("untracked file should fail");

        assert!(error
            .to_string()
            .contains("refusing to overwrite existing untracked file"));
    }

    #[test]
    fn write_generated_files_manifest_records_content_hashes() {
        let temp = TempDir::new().expect("temp dir");
        let mut files = GeneratedFiles::default();
        files.push("README.md", "generated\n");
        write_generated_files(temp.path(), files).expect("generation");

        let manifest = fs::read_to_string(temp.path().join(GENERATED_MANIFEST)).expect("manifest");

        assert!(manifest.contains("\"contentHash\""));
        assert!(manifest.contains("fnv1a64:"));
    }
}
