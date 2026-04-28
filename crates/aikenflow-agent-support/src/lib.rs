use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

const DEFAULT_AST_OUTLINE: &str = "ast-outline";
const AST_OUTLINE_ENV: &str = "AIKENFLOW_AST_OUTLINE";
const MAX_FILE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Error)]
pub enum AgentSupportError {
    #[error(
        "ast-outline is not installed or not configured. Install ast-outline and ensure it is on PATH, or set AIKENFLOW_AST_OUTLINE=/path/to/ast-outline."
    )]
    MissingAstOutline,
    #[error("ast-outline failed with status {status}: {stderr}")]
    AstOutlineFailed { status: String, stderr: String },
    #[error("failed to run ast-outline: {0}")]
    AstOutlineIo(#[source] io::Error),
    #[error("failed to read `{path}`: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write `{path}`: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("unsupported agent target `{0}`")]
    UnsupportedAgent(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstOutlineResult {
    pub files: Vec<OutlineFile>,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineFile {
    pub path: PathBuf,
    pub symbols: Vec<OutlineSymbol>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineSymbol {
    pub kind: String,
    pub name: String,
    pub signature: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
pub struct AstOutlineRunner {
    executable: OsString,
}

impl AstOutlineRunner {
    pub fn from_env() -> Self {
        let executable = std::env::var_os(AST_OUTLINE_ENV)
            .unwrap_or_else(|| OsString::from(DEFAULT_AST_OUTLINE));
        Self { executable }
    }

    pub fn new(executable: impl Into<OsString>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    pub fn run(&self, path: &Path) -> Result<AstOutlineResult, AgentSupportError> {
        let output = Command::new(&self.executable)
            .arg(path)
            .output()
            .map_err(|source| {
                if source.kind() == io::ErrorKind::NotFound {
                    AgentSupportError::MissingAstOutline
                } else {
                    AgentSupportError::AstOutlineIo(source)
                }
            })?;

        if !output.status.success() {
            return Err(AgentSupportError::AstOutlineFailed {
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }

        let raw = filter_outline_raw(String::from_utf8_lossy(&output.stdout).as_ref());
        let files = parse_ast_outline_text(&raw);
        Ok(AstOutlineResult { files, raw })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryScan {
    pub root: PathBuf,
    pub tree_entries: Vec<TreeEntry>,
    pub protocol_specs: Vec<PathBuf>,
    pub aiken_manifests: Vec<PathBuf>,
    pub aiken_files: Vec<PathBuf>,
    pub validators: Vec<DetectedSymbol>,
    pub datum_redeemer_types: Vec<DetectedSymbol>,
    pub tests: Vec<DetectedSymbol>,
    pub blueprints: Vec<PathBuf>,
    pub offchain_files: Vec<OffchainFile>,
    pub generated_artifacts: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub path: PathBuf,
    pub kind: TreeEntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedSymbol {
    pub path: PathBuf,
    pub kind: String,
    pub name: String,
    pub signature: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub fields: Vec<DetectedField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedField {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffchainFile {
    pub path: PathBuf,
    pub likely_transaction_builder: bool,
    pub markers: Vec<String>,
}

pub fn generate_outline_markdown(
    path: &Path,
    runner: &AstOutlineRunner,
) -> Result<String, AgentSupportError> {
    let scan = scan_repository(path)?;
    let outline = normalize_outline_result(runner.run(path)?, &scan.root);
    Ok(render_outline_markdown(&scan, &outline))
}

pub fn generate_agent_context_markdown(
    path: &Path,
    runner: &AstOutlineRunner,
    target: &str,
) -> Result<String, AgentSupportError> {
    if target != "codex" {
        return Err(AgentSupportError::UnsupportedAgent(target.to_owned()));
    }

    let scan = scan_repository(path)?;
    let outline = normalize_outline_result(runner.run(path)?, &scan.root);
    Ok(render_agent_context_markdown(&scan, &outline))
}

pub fn write_agent_context(
    path: &Path,
    runner: &AstOutlineRunner,
    target: &str,
) -> Result<PathBuf, AgentSupportError> {
    let markdown = generate_agent_context_markdown(path, runner, target)?;
    let root = output_root(path);
    let out = root.join(".aikenflow").join("agent-context.md");

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|source| AgentSupportError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(&out, markdown).map_err(|source| AgentSupportError::Write {
        path: out.clone(),
        source,
    })?;
    Ok(out)
}

pub fn generate_protocol_draft_markdown(path: &Path) -> Result<String, AgentSupportError> {
    let scan = scan_repository(path)?;
    Ok(render_protocol_draft_markdown(&scan))
}

pub fn write_protocol_draft(path: &Path, out: Option<&Path>) -> Result<PathBuf, AgentSupportError> {
    let markdown = generate_protocol_draft_markdown(path)?;
    let out = out.map(Path::to_path_buf).unwrap_or_else(|| {
        output_root(path)
            .join(".aikenflow")
            .join("protocol-draft.md")
    });

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|source| AgentSupportError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    fs::write(&out, markdown).map_err(|source| AgentSupportError::Write {
        path: out.clone(),
        source,
    })?;
    Ok(out)
}

pub fn scan_repository(path: &Path) -> Result<RepositoryScan, AgentSupportError> {
    let root = output_root(path);
    let mut scan = RepositoryScan {
        root: root.clone(),
        tree_entries: Vec::new(),
        protocol_specs: Vec::new(),
        aiken_manifests: Vec::new(),
        aiken_files: Vec::new(),
        validators: Vec::new(),
        datum_redeemer_types: Vec::new(),
        tests: Vec::new(),
        blueprints: Vec::new(),
        offchain_files: Vec::new(),
        generated_artifacts: Vec::new(),
        warnings: Vec::new(),
    };

    if !path.exists() {
        scan.warnings
            .push(format!("Target path `{}` does not exist.", path.display()));
        return Ok(scan);
    }

    let mut files = Vec::new();
    collect_entries(path, &root, 0, &mut scan.tree_entries, &mut files)?;

    if files.is_empty() {
        scan.warnings
            .push("No source files were found under the target path.".to_owned());
    }

    let mut unsupported_extensions = BTreeMap::<String, usize>::new();

    for file in files {
        let relative = relative_path(&file, &root);
        let Some(extension) = file.extension().and_then(OsStr::to_str) else {
            continue;
        };
        let extension = extension.to_ascii_lowercase();

        match extension.as_str() {
            "ak" => {
                scan.aiken_files.push(relative.clone());
                inspect_aiken_file(&file, &relative, &mut scan)?;
            }
            "json" => {
                if is_blueprint_path(&relative) {
                    scan.blueprints.push(relative.clone());
                }
            }
            "yaml" | "yml" => {
                if is_protocol_spec_path(&relative) {
                    scan.protocol_specs.push(relative.clone());
                }
            }
            "toml" => {
                if relative.file_name().and_then(OsStr::to_str) == Some("aiken.toml") {
                    scan.aiken_manifests.push(relative.clone());
                }
            }
            "ts" | "tsx" | "js" | "mjs" => {
                inspect_offchain_file(&file, &relative, &mut scan)?;
            }
            "md" | "lock" | "mmd" => {}
            other => {
                *unsupported_extensions.entry(other.to_owned()).or_default() += 1;
            }
        }

        if is_generated_artifact_path(&relative) {
            scan.generated_artifacts.push(relative);
        }
    }

    if scan.aiken_files.is_empty() {
        scan.warnings.push(
            "No `.ak` files detected. Existing-project Aiken analysis will be limited.".to_owned(),
        );
    }

    for (extension, count) in unsupported_extensions {
        scan.warnings.push(format!(
            "Ignored {count} unsupported `.{extension}` file(s) during heuristic scanning."
        ));
    }

    dedup_paths(&mut scan.aiken_files);
    dedup_paths(&mut scan.protocol_specs);
    dedup_paths(&mut scan.aiken_manifests);
    dedup_paths(&mut scan.blueprints);
    dedup_paths(&mut scan.generated_artifacts);
    Ok(scan)
}

pub fn parse_ast_outline_text(raw: &str) -> Vec<OutlineFile> {
    let mut files = Vec::new();
    let mut current: Option<OutlineFile> = None;

    for line in raw.lines() {
        if let Some(path) = parse_file_header(line) {
            if let Some(file) = current.take() {
                files.push(file);
            }
            current = Some(OutlineFile {
                path: PathBuf::from(path),
                symbols: Vec::new(),
            });
            continue;
        }

        if let Some(symbol) = parse_symbol_line(line) {
            if let Some(file) = &mut current {
                file.symbols.push(symbol);
            }
        }
    }

    if let Some(file) = current {
        files.push(file);
    }

    files
}

fn render_outline_markdown(scan: &RepositoryScan, outline: &AstOutlineResult) -> String {
    let mut out = String::new();
    out.push_str("# AikenFlow Repository Outline\n\n");
    out.push_str("Boundary: `ast-outline` is used for navigation, code maps, and agent context only. AikenFlow IR remains the source of protocol semantics, invariants, and generation.\n\n");
    push_project_summary(&mut out, scan, outline);
    push_tree_summary(&mut out, scan);
    push_detected_aiken(&mut out, scan);
    push_offchain_builders(&mut out, scan);
    push_generated_artifacts(&mut out, scan);
    push_ast_outline(&mut out, outline);
    push_warnings(&mut out, scan, outline);
    out
}

fn render_agent_context_markdown(scan: &RepositoryScan, outline: &AstOutlineResult) -> String {
    let mut out = String::new();
    out.push_str("# AikenFlow Agent Context\n\n");
    out.push_str("Boundary: `ast-outline` is navigation / code map / agent context. `AikenFlow IR` is protocol semantics / invariants / generation.\n\n");
    out.push_str("## Project Summary\n\n");
    push_project_summary_body(&mut out, scan, outline);
    push_tree_summary(&mut out, scan);
    push_detected_aiken(&mut out, scan);
    push_offchain_builders(&mut out, scan);
    push_generated_artifacts(&mut out, scan);
    push_suggested_next_files(&mut out, scan);
    push_suggested_next_edit_points(&mut out, scan, outline);
    push_ast_outline_compact(&mut out, outline);
    push_warnings(&mut out, scan, outline);
    out
}

fn render_protocol_draft_markdown(scan: &RepositoryScan) -> String {
    let mut out = String::new();
    out.push_str("# AikenFlow Existing Project Protocol Draft\n\n");
    out.push_str("THIS IS A DRAFT. NOT VERIFIED.\n\n");
    out.push_str("Status: DRAFT ONLY. This artefact is not an AikenFlow protocol spec, does not validate transitions, and must not be used as compiler correctness evidence.\n\n");
    out.push_str("Boundary: repository heuristics can identify candidate files and symbols. `AikenFlow IR` remains the only source for protocol semantics, invariants, generation, `check`, `gen`, `graph`, and `audit`.\n\n");
    push_protocol_draft_summary(&mut out, scan);
    push_existing_protocol_specs(&mut out, scan);
    push_protocol_draft_candidates(&mut out, scan);
    push_protocol_draft_sketch(&mut out, scan);
    push_generated_artifacts(&mut out, scan);
    push_suggested_next_files(&mut out, scan);
    push_protocol_draft_review_steps(&mut out);
    push_scan_warnings(&mut out, scan);
    out
}

fn push_project_summary(out: &mut String, scan: &RepositoryScan, outline: &AstOutlineResult) {
    out.push_str("## Project Summary\n\n");
    push_project_summary_body(out, scan, outline);
}

fn push_project_summary_body(out: &mut String, scan: &RepositoryScan, outline: &AstOutlineResult) {
    out.push_str(&format!("- Root: `{}`\n", scan.root.display()));
    out.push_str(&format!(
        "- Tree entries scanned: {}\n",
        scan.tree_entries.len()
    ));
    out.push_str(&format!(
        "- Protocol specs: {}\n",
        scan.protocol_specs.len()
    ));
    out.push_str(&format!(
        "- Aiken manifests: {}\n",
        scan.aiken_manifests.len()
    ));
    out.push_str(&format!("- Aiken files: {}\n", scan.aiken_files.len()));
    out.push_str(&format!("- Validators: {}\n", scan.validators.len()));
    out.push_str(&format!(
        "- Datum/redeemer-like types: {}\n",
        scan.datum_redeemer_types.len()
    ));
    out.push_str(&format!(
        "- Likely off-chain transaction builders: {}\n",
        scan.offchain_files
            .iter()
            .filter(|file| file.likely_transaction_builder)
            .count()
    ));
    out.push_str(&format!("- ast-outline files: {}\n\n", outline.files.len()));
}

fn push_protocol_draft_summary(out: &mut String, scan: &RepositoryScan) {
    out.push_str("## Project Summary\n\n");
    out.push_str(&format!("- Root: `{}`\n", scan.root.display()));
    out.push_str(&format!(
        "- Tree entries scanned: {}\n",
        scan.tree_entries.len()
    ));
    out.push_str(&format!(
        "- Protocol specs detected: {}\n",
        scan.protocol_specs.len()
    ));
    out.push_str(&format!(
        "- Aiken manifests detected: {}\n",
        scan.aiken_manifests.len()
    ));
    out.push_str(&format!(
        "- Candidate validators: {}\n",
        scan.validators.len()
    ));
    out.push_str(&format!(
        "- Candidate datum/redeemer-like types: {}\n",
        scan.datum_redeemer_types.len()
    ));
    out.push_str(&format!(
        "- Candidate off-chain builders: {}\n\n",
        scan.offchain_files
            .iter()
            .filter(|file| file.likely_transaction_builder)
            .count()
    ));
}

fn push_existing_protocol_specs(out: &mut String, scan: &RepositoryScan) {
    out.push_str("## Existing Protocol Specs\n\n");
    if scan.protocol_specs.is_empty() {
        out.push_str("_No existing AikenFlow protocol specs detected. Create a reviewed `protocol.yaml` before using compiler commands._\n\n");
        return;
    }

    for path in &scan.protocol_specs {
        out.push_str(&format!("- `{}`\n", path.display()));
    }
    out.push_str("\nReview the detected spec before running `aikenflow check`, `gen`, `graph`, or `audit`.\n\n");
}

fn push_protocol_draft_candidates(out: &mut String, scan: &RepositoryScan) {
    out.push_str("## Candidate Validators\n\n");
    push_symbols(
        out,
        &scan.validators,
        "_No candidate validator declarations detected._",
    );

    out.push_str("## Candidate Datum / Redeemer / Policy / State Types\n\n");
    push_symbols(
        out,
        &scan.datum_redeemer_types,
        "_No candidate datum/redeemer-like types detected._",
    );

    out.push_str("## Candidate Off-chain Transaction Builders\n\n");
    let builders = scan
        .offchain_files
        .iter()
        .filter(|file| file.likely_transaction_builder)
        .collect::<Vec<_>>();

    if builders.is_empty() {
        out.push_str("_No candidate off-chain transaction builders detected._\n\n");
        return;
    }

    for file in builders {
        out.push_str(&format!(
            "- `{}` markers: {}\n",
            file.path.display(),
            file.markers.join(", ")
        ));
    }
    out.push('\n');
}

fn push_protocol_draft_sketch(out: &mut String, scan: &RepositoryScan) {
    out.push_str("## Draft Protocol Sketch\n\n");
    out.push_str("This sketch is intentionally incomplete. Rewrite it into a user-owned protocol spec before compiler use.\n\n");
    out.push_str("```yaml\n");
    out.push_str("# DRAFT ONLY - manually review and rewrite before `aikenflow check`.\n");
    out.push_str(&format!("protocol: {}\n", draft_protocol_name(scan)));
    out.push_str("description: \"Draft inferred from repository navigation heuristics.\"\n\n");
    out.push_str("states:\n");
    let states = candidate_states(scan);
    if states.is_empty() {
        out.push_str("  # TODO: define protocol states and datum fields.\n");
    } else {
        for state in states {
            out.push_str(&format!(
                "  # Candidate from `{}`.\n",
                state.source.display()
            ));
            out.push_str(&format!("  {}:\n", state.name));
            out.push_str("    datum:\n");
            if state.fields.is_empty() {
                out.push_str("      # TODO: copy reviewed datum fields from source Aiken code.\n");
            } else {
                for field in state.fields {
                    out.push_str(&format!("      {}: {}\n", field.name, field.ty));
                }
                out.push_str(
                    "      # TODO: review inferred fields and Aiken/Cardano type mappings.\n",
                );
            }
        }
    }

    out.push_str("\ntransitions:\n");
    if scan.validators.is_empty() {
        out.push_str("  # TODO: define transitions from reviewed validator and off-chain flows.\n");
    } else {
        for validator in &scan.validators {
            let transition = to_pascal_case(&validator.name);
            out.push_str(&format!(
                "  # Candidate from validator `{}` in `{}` L{}-{}.\n",
                validator.name,
                validator.path.display(),
                validator.start_line,
                validator.end_line
            ));
            out.push_str(&format!("  {transition}:\n"));
            out.push_str("    inputs:\n");
            out.push_str("      # TODO: map consumed states after manual review.\n");
            out.push_str("    outputs:\n");
            out.push_str("      # TODO: map produced states after manual review.\n");
            out.push_str("    constraints:\n");
            out.push_str("      # TODO: encode explicit signatures, time bounds, value rules, and datum checks.\n");
        }
    }

    out.push_str("\ninvariants:\n");
    out.push_str("  # TODO: add human-reviewed protocol invariants.\n");
    out.push_str("```\n\n");
}

fn push_protocol_draft_review_steps(out: &mut String) {
    out.push_str("## Required Review Before Compiler Use\n\n");
    out.push_str(
        "- Write or update `protocol.yaml` by hand from the candidate information above.\n",
    );
    out.push_str("- Run `aikenflow check protocol.yaml` and address all validation errors.\n");
    out.push_str("- Only then run `aikenflow gen`, `graph`, or `audit` from the reviewed spec.\n");
    out.push_str(
        "- Do not treat this draft as proof of validator behavior or protocol invariants.\n\n",
    );
}

fn push_scan_warnings(out: &mut String, scan: &RepositoryScan) {
    out.push_str("## Warnings\n\n");
    if scan.warnings.is_empty() {
        out.push_str("_No warnings._\n");
        return;
    }

    for warning in &scan.warnings {
        out.push_str(&format!("- {warning}\n"));
    }
}

fn push_tree_summary(out: &mut String, scan: &RepositoryScan) {
    out.push_str("## Project Tree Summary\n\n");
    if scan.tree_entries.is_empty() {
        out.push_str("_No project tree entries detected._\n\n");
        return;
    }

    for entry in scan.tree_entries.iter().take(80) {
        let marker = match entry.kind {
            TreeEntryKind::Directory => "/",
            TreeEntryKind::File => "",
        };
        out.push_str(&format!("- `{}{}`\n", entry.path.display(), marker));
    }

    if scan.tree_entries.len() > 80 {
        out.push_str(&format!(
            "- _{} additional entries omitted._\n",
            scan.tree_entries.len() - 80
        ));
    }
    out.push('\n');
}

fn push_detected_aiken(out: &mut String, scan: &RepositoryScan) {
    out.push_str("## Protocol Specs\n\n");
    if scan.protocol_specs.is_empty() {
        out.push_str("_No AikenFlow protocol specs detected._\n\n");
    } else {
        for path in &scan.protocol_specs {
            out.push_str(&format!("- `{}`\n", path.display()));
        }
        out.push('\n');
    }

    out.push_str("## Aiken Manifests\n\n");
    if scan.aiken_manifests.is_empty() {
        out.push_str("_No `aiken.toml` manifests detected._\n\n");
    } else {
        for path in &scan.aiken_manifests {
            out.push_str(&format!("- `{}`\n", path.display()));
        }
        out.push('\n');
    }

    out.push_str("## Aiken Files\n\n");
    if scan.aiken_files.is_empty() {
        out.push_str("_No `.ak` files detected._\n\n");
    } else {
        for path in &scan.aiken_files {
            out.push_str(&format!("- `{}`\n", path.display()));
        }
        out.push('\n');
    }

    out.push_str("## Validators\n\n");
    push_symbols(
        out,
        &scan.validators,
        "_No validator declarations detected._",
    );

    out.push_str("## Datum / Redeemer Types\n\n");
    push_symbols(
        out,
        &scan.datum_redeemer_types,
        "_No datum/redeemer-like types detected._",
    );

    out.push_str("## Tests\n\n");
    push_symbols(out, &scan.tests, "_No Aiken test declarations detected._");

    out.push_str("## Blueprints\n\n");
    if scan.blueprints.is_empty() {
        out.push_str("_No blueprint files detected._\n\n");
    } else {
        for path in &scan.blueprints {
            out.push_str(&format!("- `{}`\n", path.display()));
        }
        out.push('\n');
    }
}

fn push_symbols(out: &mut String, symbols: &[DetectedSymbol], empty: &str) {
    if symbols.is_empty() {
        out.push_str(empty);
        out.push_str("\n\n");
        return;
    }

    for symbol in symbols {
        let signature = symbol
            .signature
            .as_deref()
            .unwrap_or(symbol.name.as_str())
            .trim();
        out.push_str(&format!(
            "- `{}` `{}` in `{}` L{}-{}\n",
            symbol.kind,
            signature,
            symbol.path.display(),
            symbol.start_line,
            symbol.end_line
        ));
    }
    out.push('\n');
}

fn push_offchain_builders(out: &mut String, scan: &RepositoryScan) {
    out.push_str("## Off-chain Transaction Builders\n\n");
    let builders = scan
        .offchain_files
        .iter()
        .filter(|file| file.likely_transaction_builder)
        .collect::<Vec<_>>();

    if builders.is_empty() {
        out.push_str("_No likely off-chain transaction builder files detected._\n\n");
        return;
    }

    for file in builders {
        out.push_str(&format!(
            "- `{}` markers: {}\n",
            file.path.display(),
            file.markers.join(", ")
        ));
    }
    out.push('\n');
}

fn push_generated_artifacts(out: &mut String, scan: &RepositoryScan) {
    out.push_str("## Generated Artefacts\n\n");
    if scan.generated_artifacts.is_empty() {
        out.push_str("_No generated artefact locations detected._\n\n");
        return;
    }

    for path in scan.generated_artifacts.iter().take(50) {
        out.push_str(&format!("- `{}`\n", path.display()));
    }
    if scan.generated_artifacts.len() > 50 {
        out.push_str(&format!(
            "- _{} additional generated artefacts omitted._\n",
            scan.generated_artifacts.len() - 50
        ));
    }
    out.push('\n');
}

fn push_suggested_next_files(out: &mut String, scan: &RepositoryScan) {
    out.push_str("## Suggested Next Files To Inspect\n\n");
    let mut files = Vec::<PathBuf>::new();
    for spec in &scan.protocol_specs {
        push_unique_path(&mut files, spec.clone());
    }
    for validator in &scan.validators {
        push_unique_path(&mut files, validator.path.clone());
    }
    for ty in &scan.datum_redeemer_types {
        push_unique_path(&mut files, ty.path.clone());
    }
    for file in scan
        .offchain_files
        .iter()
        .filter(|file| file.likely_transaction_builder)
    {
        push_unique_path(&mut files, file.path.clone());
    }
    for blueprint in &scan.blueprints {
        push_unique_path(&mut files, blueprint.clone());
    }
    for manifest in &scan.aiken_manifests {
        push_unique_path(&mut files, manifest.clone());
    }

    if files.is_empty() {
        out.push_str("_No high-confidence next files detected._\n\n");
        return;
    }

    for file in files.into_iter().take(12) {
        out.push_str(&format!("- `{}`\n", file.display()));
    }
    out.push('\n');
}

fn push_suggested_next_edit_points(
    out: &mut String,
    scan: &RepositoryScan,
    outline: &AstOutlineResult,
) {
    out.push_str("## Suggested Next Edit Points\n\n");
    let mut points = Vec::<String>::new();

    for validator in &scan.validators {
        push_unique_string(
            &mut points,
            format!(
                "- `{}:{}` validator `{}`\n",
                validator.path.display(),
                validator.start_line,
                validator.name
            ),
        );
    }

    for ty in &scan.datum_redeemer_types {
        push_unique_string(
            &mut points,
            format!(
                "- `{}:{}` type `{}`\n",
                ty.path.display(),
                ty.start_line,
                ty.name
            ),
        );
    }

    for file in scan
        .offchain_files
        .iter()
        .filter(|file| file.likely_transaction_builder)
    {
        push_unique_string(
            &mut points,
            format!(
                "- `{}` transaction builder candidate; markers: {}\n",
                file.path.display(),
                file.markers.join(", ")
            ),
        );
    }

    for file in &outline.files {
        for symbol in &file.symbols {
            if matches!(symbol.kind.as_str(), "function" | "type") {
                push_unique_string(
                    &mut points,
                    format!(
                        "- `{}:{}` `{}` `{}`\n",
                        file.path.display(),
                        symbol.start_line,
                        symbol.kind,
                        symbol.name
                    ),
                );
            }
        }
    }

    if points.is_empty() {
        out.push_str("_No high-confidence edit points detected._\n\n");
        return;
    }

    for point in points.into_iter().take(12) {
        out.push_str(&point);
    }
    out.push('\n');
}

fn push_ast_outline(out: &mut String, outline: &AstOutlineResult) {
    out.push_str("## AST Outline Summary\n\n");
    if outline.files.is_empty() && outline.raw.trim().is_empty() {
        out.push_str("_ast-outline returned no symbols for this target._\n\n");
        return;
    }

    if !outline.files.is_empty() {
        push_outline_files(out, outline, 80, 20);
        return;
    }

    out.push_str("```text\n");
    out.push_str(outline.raw.trim());
    out.push_str("\n```\n\n");
}

fn push_ast_outline_compact(out: &mut String, outline: &AstOutlineResult) {
    out.push_str("## AST Outline Summary\n\n");
    if outline.files.is_empty() {
        out.push_str("_ast-outline returned no structured symbols. Raw output may be empty or unsupported for these file types._\n\n");
        return;
    }

    push_outline_files(out, outline, 30, 12);
}

fn push_outline_files(
    out: &mut String,
    outline: &AstOutlineResult,
    file_limit: usize,
    symbol_limit: usize,
) {
    for file in outline.files.iter().take(file_limit) {
        out.push_str(&format!("- `{}`\n", file.path.display()));
        for symbol in file.symbols.iter().take(symbol_limit) {
            out.push_str(&format!(
                "  - `{}` `{}` L{}-{}\n",
                symbol.kind,
                symbol
                    .signature
                    .as_deref()
                    .unwrap_or(symbol.name.as_str())
                    .trim(),
                symbol.start_line,
                symbol.end_line
            ));
        }
        if file.symbols.len() > symbol_limit {
            out.push_str(&format!(
                "  - _{} additional symbols omitted._\n",
                file.symbols.len() - symbol_limit
            ));
        }
    }
    if outline.files.len() > file_limit {
        out.push_str(&format!(
            "- _{} additional outline files omitted._\n",
            outline.files.len() - file_limit
        ));
    }
    out.push('\n');
}

fn push_warnings(out: &mut String, scan: &RepositoryScan, outline: &AstOutlineResult) {
    out.push_str("## Warnings\n\n");
    let mut warnings = scan.warnings.clone();

    if outline.raw.trim().is_empty() {
        warnings.push("ast-outline returned an empty outline. AikenFlow heuristic scanning was still applied.".to_owned());
    }

    if warnings.is_empty() {
        out.push_str("_No warnings._\n");
        return;
    }

    for warning in warnings {
        out.push_str(&format!("- {warning}\n"));
    }
}

fn collect_entries(
    path: &Path,
    root: &Path,
    depth: usize,
    tree: &mut Vec<TreeEntry>,
    files: &mut Vec<PathBuf>,
) -> Result<(), AgentSupportError> {
    if depth > 8 {
        return Ok(());
    }

    if path.is_file() {
        let relative = relative_path(path, root);
        tree.push(TreeEntry {
            path: relative,
            kind: TreeEntryKind::File,
        });
        files.push(path.to_path_buf());
        return Ok(());
    }

    let mut entries = fs::read_dir(path)
        .map_err(|source| AgentSupportError::Read {
            path: path.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| AgentSupportError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if should_skip_entry(&file_name) {
            continue;
        }

        let relative = relative_path(&entry_path, root);
        let file_type = entry
            .file_type()
            .map_err(|source| AgentSupportError::Read {
                path: entry_path.clone(),
                source,
            })?;

        if file_type.is_dir() {
            tree.push(TreeEntry {
                path: relative,
                kind: TreeEntryKind::Directory,
            });
            collect_entries(&entry_path, root, depth + 1, tree, files)?;
        } else if file_type.is_file() {
            tree.push(TreeEntry {
                path: relative,
                kind: TreeEntryKind::File,
            });
            files.push(entry_path);
        }
    }

    Ok(())
}

fn should_skip_entry(file_name: &str) -> bool {
    matches!(
        file_name,
        ".git" | "target" | "node_modules" | ".aikenflow" | "build" | "dist" | ".next"
    )
}

fn is_support_or_build_path(path: &Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        matches!(
            name.as_ref(),
            ".aikenflow" | ".git" | "target" | "node_modules" | "build" | "dist" | ".next"
        )
    })
}

fn inspect_aiken_file(
    path: &Path,
    relative: &Path,
    scan: &mut RepositoryScan,
) -> Result<(), AgentSupportError> {
    let Some(content) = read_small_text_file(path, relative, scan)? else {
        return Ok(());
    };
    let lines = content.lines().collect::<Vec<_>>();

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let line_number = index + 1;

        if let Some(name) = parse_aiken_validator_name(trimmed) {
            scan.validators.push(DetectedSymbol {
                path: relative.to_path_buf(),
                kind: "validator".to_owned(),
                name: name.clone(),
                signature: Some(trimmed.to_owned()),
                start_line: line_number,
                end_line: find_block_end(&lines, index),
                fields: Vec::new(),
            });
        }

        if let Some(name) = parse_aiken_type_name(trimmed) {
            if is_datum_redeemer_like_type(&name) {
                let end_line = find_block_end(&lines, index);
                scan.datum_redeemer_types.push(DetectedSymbol {
                    path: relative.to_path_buf(),
                    kind: "type".to_owned(),
                    name,
                    signature: Some(trimmed.to_owned()),
                    start_line: line_number,
                    end_line,
                    fields: parse_aiken_type_fields(&lines, index, end_line),
                });
            }
        }

        if let Some(name) = parse_aiken_test_name(trimmed) {
            scan.tests.push(DetectedSymbol {
                path: relative.to_path_buf(),
                kind: "test".to_owned(),
                name,
                signature: Some(trimmed.to_owned()),
                start_line: line_number,
                end_line: find_block_end(&lines, index),
                fields: Vec::new(),
            });
        }
    }

    Ok(())
}

fn inspect_offchain_file(
    path: &Path,
    relative: &Path,
    scan: &mut RepositoryScan,
) -> Result<(), AgentSupportError> {
    let mut markers = Vec::new();
    let file_name = relative
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    for marker in OFFCHAIN_MARKERS {
        if file_name.contains(&marker.to_ascii_lowercase()) {
            markers.push((*marker).to_owned());
        }
    }

    let content = read_small_text_file(path, relative, scan)?;
    if let Some(content) = &content {
        let lower = content.to_ascii_lowercase();
        for marker in OFFCHAIN_MARKERS {
            if lower.contains(&marker.to_ascii_lowercase()) && !markers.iter().any(|m| m == marker)
            {
                markers.push((*marker).to_owned());
            }
        }
    }

    let has_builder_marker = markers.iter().any(|marker| is_builder_marker(marker));
    let likely_transaction_builder = !markers.is_empty()
        && (!is_test_like_offchain_path(relative) || has_builder_marker)
        && !is_type_support_offchain_path(relative, content.as_deref());
    scan.offchain_files.push(OffchainFile {
        path: relative.to_path_buf(),
        likely_transaction_builder,
        markers,
    });

    Ok(())
}

const OFFCHAIN_MARKERS: &[&str] = &[
    "lucid",
    "mesh",
    "blaze",
    "newTx",
    "collectFrom",
    "payToContract",
    "attachSpendingValidator",
    "mintAssets",
];

fn is_builder_marker(marker: &str) -> bool {
    !matches!(marker, "lucid" | "mesh" | "blaze")
}

fn is_test_like_offchain_path(path: &Path) -> bool {
    let text = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    text.contains("/test/")
        || text.contains("/tests/")
        || text.contains(".spec.")
        || text.contains(".test.")
}

fn is_type_support_offchain_path(path: &Path, content: Option<&str>) -> bool {
    let text = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let looks_like_type_support = text.ends_with("/types.ts")
        || text.ends_with("/types.tsx")
        || text.ends_with("/types.js")
        || text.ends_with("/types.mjs")
        || text.contains("/types/");

    looks_like_type_support
        && !content
            .map(has_transaction_builder_call)
            .unwrap_or_default()
}

fn has_transaction_builder_call(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    [
        ".newtx(",
        ".collectfrom(",
        ".paytocontract(",
        ".attachspendingvalidator(",
        ".mintassets(",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn read_small_text_file(
    path: &Path,
    relative: &Path,
    scan: &mut RepositoryScan,
) -> Result<Option<String>, AgentSupportError> {
    let metadata = fs::metadata(path).map_err(|source| AgentSupportError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    if metadata.len() > MAX_FILE_BYTES {
        scan.warnings.push(format!(
            "Skipped `{}` because it is larger than {} bytes.",
            relative.display(),
            MAX_FILE_BYTES
        ));
        return Ok(None);
    }

    let bytes = fs::read(path).map_err(|source| AgentSupportError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    match String::from_utf8(bytes) {
        Ok(content) => Ok(Some(content)),
        Err(_) => {
            scan.warnings
                .push(format!("Skipped non-UTF-8 file `{}`.", relative.display()));
            Ok(None)
        }
    }
}

fn parse_aiken_validator_name(line: &str) -> Option<String> {
    parse_keyword_name(line, "validator")
}

fn parse_aiken_type_name(line: &str) -> Option<String> {
    parse_keyword_name(line.strip_prefix("pub ").unwrap_or(line), "type")
}

fn parse_aiken_test_name(line: &str) -> Option<String> {
    parse_keyword_name(line, "test")
}

fn parse_aiken_type_fields(
    lines: &[&str],
    start_index: usize,
    end_line: usize,
) -> Vec<DetectedField> {
    let mut fields = Vec::new();
    let end_index = end_line.min(lines.len());

    for (offset, line) in lines[start_index..end_index].iter().enumerate() {
        let mut text = line.trim();
        if offset == 0 {
            let Some((_, after_open)) = text.split_once('{') else {
                continue;
            };
            text = after_open.trim();
        }
        if let Some((before_close, _)) = text.split_once('}') {
            text = before_close.trim();
        }

        for segment in text.split(',') {
            if let Some(field) = parse_aiken_type_field(segment) {
                fields.push(field);
            }
        }
    }

    fields
}

fn parse_aiken_type_field(segment: &str) -> Option<DetectedField> {
    let segment = segment
        .split("//")
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches(',');
    let (name, ty) = segment.split_once(':')?;
    let name = name.trim();
    let ty = ty.trim();

    if name.is_empty() || ty.is_empty() || name.chars().any(char::is_whitespace) {
        return None;
    }

    Some(DetectedField {
        name: name.to_owned(),
        ty: ty.to_owned(),
    })
}

fn parse_keyword_name(line: &str, keyword: &str) -> Option<String> {
    let remainder = line.strip_prefix(keyword)?.trim_start();
    if remainder.is_empty() {
        return None;
    }
    let name = remainder
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '(' | '{' | ':' | '<')
        })
        .next()
        .unwrap_or_default();
    (!name.is_empty()).then(|| name.to_owned())
}

fn is_datum_redeemer_like_type(name: &str) -> bool {
    ["Datum", "Redeemer", "Policy", "State"]
        .iter()
        .any(|needle| name.contains(needle))
}

fn find_block_end(lines: &[&str], start_index: usize) -> usize {
    let mut depth: isize = 0;
    let mut seen_open = false;

    for (offset, line) in lines[start_index..].iter().enumerate() {
        for character in line.chars() {
            match character {
                '{' => {
                    depth += 1;
                    seen_open = true;
                }
                '}' => {
                    depth -= 1;
                    if seen_open && depth <= 0 {
                        return start_index + offset + 1;
                    }
                }
                _ => {}
            }
        }
    }

    start_index + 1
}

fn parse_file_header(line: &str) -> Option<String> {
    let line = line.trim();
    let rest = line.strip_prefix("# ")?;
    let (path, _) = rest.split_once(" (")?;
    if path.is_empty() {
        None
    } else {
        Some(path.to_owned())
    }
}

fn parse_symbol_line(line: &str) -> Option<OutlineSymbol> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (before_line, range) = trimmed.rsplit_once("  L")?;
    let (start_line, end_line) = parse_line_range(range)?;
    let (kind, name, signature) = classify_outline_signature(before_line);

    Some(OutlineSymbol {
        kind,
        name,
        signature: Some(signature),
        start_line,
        end_line,
    })
}

fn parse_line_range(range: &str) -> Option<(usize, usize)> {
    if let Some((start, end)) = range.split_once('-') {
        Some((start.parse().ok()?, end.parse().ok()?))
    } else {
        let line = range.parse().ok()?;
        Some((line, line))
    }
}

fn filter_outline_raw(raw: &str) -> String {
    let mut filtered = String::new();
    let mut include_block = true;
    let mut in_file_block = false;

    for line in raw.lines() {
        if let Some(path) = parse_file_header(line) {
            in_file_block = true;
            include_block = !is_support_or_build_path(Path::new(&path));
            if include_block {
                filtered.push_str(line);
                filtered.push('\n');
            }
            continue;
        }

        if !include_block {
            continue;
        }

        if in_file_block && !line.trim().is_empty() && parse_symbol_line(line).is_none() {
            continue;
        }

        filtered.push_str(line);
        filtered.push('\n');
    }

    filtered.trim_end().to_owned()
}

fn normalize_outline_result(mut outline: AstOutlineResult, root: &Path) -> AstOutlineResult {
    for file in &mut outline.files {
        file.path = normalize_outline_path(&file.path, root);
    }
    outline.raw = normalize_outline_raw(&outline.raw, root);
    outline
}

fn normalize_outline_raw(raw: &str, root: &Path) -> String {
    let mut normalized = String::new();

    for line in raw.lines() {
        if let Some(path) = parse_file_header(line) {
            let normalized_path = normalize_outline_path(Path::new(&path), root);
            let suffix = line
                .trim()
                .strip_prefix("# ")
                .and_then(|rest| rest.split_once(" (").map(|(_, suffix)| suffix));
            if let Some(suffix) = suffix {
                normalized.push_str(&format!("# {} ({suffix}\n", normalized_path.display()));
            } else {
                normalized.push_str(line);
                normalized.push('\n');
            }
        } else {
            normalized.push_str(line);
            normalized.push('\n');
        }
    }

    normalized.trim_end().to_owned()
}

fn normalize_outline_path(path: &Path, root: &Path) -> PathBuf {
    if let Ok(relative) = path.strip_prefix(root) {
        return relative.to_path_buf();
    }

    let path_text = path.to_string_lossy().replace('\\', "/");
    let root_text = root.to_string_lossy().replace('\\', "/");
    if let Some(relative) = path_text.strip_prefix(&(root_text + "/")) {
        return PathBuf::from(relative);
    }

    path.to_path_buf()
}

fn classify_outline_signature(signature: &str) -> (String, String, String) {
    let signature = signature.trim().to_owned();
    let mut trimmed = signature.as_str();
    if let Some(rest) = trimmed.strip_prefix("export ") {
        trimmed = rest;
    }
    if let Some(rest) = trimmed.strip_prefix("async ") {
        trimmed = rest;
    }

    if let Some(rest) = trimmed.strip_prefix("function ") {
        let name = rest
            .split(|character: char| character == '(' || character.is_whitespace())
            .next()
            .unwrap_or("function")
            .to_owned();
        return ("function".to_owned(), name, signature);
    }

    if let Some(rest) = trimmed.strip_prefix("interface ") {
        let name = rest
            .split_whitespace()
            .next()
            .unwrap_or("interface")
            .to_owned();
        return ("type".to_owned(), name, signature);
    }

    if let Some(rest) = trimmed.strip_prefix("type ") {
        let name = rest
            .split(|character: char| character == '=' || character.is_whitespace())
            .next()
            .unwrap_or("type")
            .to_owned();
        return ("type".to_owned(), name, signature);
    }

    if signature.contains(" code block") {
        return ("code-block".to_owned(), "code block".to_owned(), signature);
    }

    if signature.contains(':') {
        let name = signature
            .split(':')
            .next()
            .unwrap_or("field")
            .trim()
            .to_owned();
        return ("field".to_owned(), name, signature);
    }

    ("symbol".to_owned(), signature.clone(), signature)
}

fn is_blueprint_path(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    file_name == "plutus.json" || file_name.contains("blueprint")
}

fn is_protocol_spec_path(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        file_name.as_str(),
        "protocol.yaml" | "protocol.yml" | "aikenflow.yaml" | "aikenflow.yml"
    )
}

fn is_generated_artifact_path(path: &Path) -> bool {
    let text = path.to_string_lossy().replace('\\', "/");
    text.contains("/generated/")
        || text.starts_with("generated/")
        || text.ends_with("/.aikenflow-generated.json")
        || text == ".aikenflow-generated.json"
        || text.ends_with("/AUDIT.md")
        || text == "AUDIT.md"
        || text.ends_with("/state-graph.mmd")
        || text.ends_with("/topology.json")
        || text.ends_with("/invariant-matrix.md")
}

fn output_root(path: &Path) -> PathBuf {
    if path.is_file() {
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    } else {
        path.to_path_buf()
    }
}

fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn dedup_paths(paths: &mut Vec<PathBuf>) {
    let mut seen = BTreeSet::new();
    paths.retain(|path| seen.insert(path.clone()));
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CandidateState {
    name: String,
    source: PathBuf,
    fields: Vec<DetectedField>,
}

fn draft_protocol_name(scan: &RepositoryScan) -> String {
    scan.root
        .file_name()
        .and_then(OsStr::to_str)
        .map(to_pascal_case)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "DraftProtocol".to_owned())
}

fn candidate_states(scan: &RepositoryScan) -> Vec<CandidateState> {
    let mut seen = BTreeSet::new();
    let mut states = Vec::new();

    for symbol in &scan.datum_redeemer_types {
        let Some(state) = candidate_state_name(&symbol.name) else {
            continue;
        };
        if seen.insert(state.clone()) {
            states.push(CandidateState {
                name: state,
                source: symbol.path.clone(),
                fields: symbol.fields.clone(),
            });
        }
    }

    states
}

fn candidate_state_name(type_name: &str) -> Option<String> {
    for suffix in ["Datum", "State"] {
        if let Some(base) = type_name.strip_suffix(suffix) {
            let state = to_pascal_case(base);
            if !state.is_empty() {
                return Some(state);
            }
        }
    }
    None
}

fn to_pascal_case(input: &str) -> String {
    let mut out = String::new();
    let mut capitalize_next = true;

    for character in input.chars() {
        if character.is_ascii_alphanumeric() {
            if capitalize_next {
                out.push(character.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                out.push(character);
            }
        } else {
            capitalize_next = true;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ast_outline_default_text() {
        let raw = r#"# src/index.ts (12 lines)
export interface Params  L1-3
export async function withdraw(params: Params): Promise<Tx>  L5-10
"#;

        let parsed = parse_ast_outline_text(raw);

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].path, PathBuf::from("src/index.ts"));
        assert_eq!(parsed[0].symbols.len(), 2);
        assert_eq!(parsed[0].symbols[0].kind, "type");
        assert_eq!(parsed[0].symbols[0].name, "Params");
        assert_eq!(parsed[0].symbols[1].kind, "function");
        assert_eq!(parsed[0].symbols[1].name, "withdraw");
        assert_eq!(parsed[0].symbols[1].start_line, 5);
    }

    #[test]
    fn filters_support_paths_from_outline_raw() {
        let raw = r#"# .aikenflow/agent-context.md (12 lines)
# AikenFlow Agent Context  L1-12

# src/index.ts (4 lines)
export function build(): Tx  L1-4
"#;

        let filtered = filter_outline_raw(raw);

        assert!(!filtered.contains(".aikenflow/agent-context.md"));
        assert!(filtered.contains("src/index.ts"));
    }

    #[test]
    fn filters_source_body_lines_from_outline_raw() {
        let raw = r#"# src/index.ts (8 lines)
export function build(): Tx  L1-6
  const tx = lucid.newTx()
  return tx.complete()
}
"#;

        let filtered = filter_outline_raw(raw);

        assert!(filtered.contains("export function build(): Tx  L1-6"));
        assert!(!filtered.contains("const tx"));
        assert!(!filtered.contains("return tx"));
        assert!(!filtered.contains("}"));
    }
}
