use aikenflow_ir::{
    Constraint, Diagnostic, DiagnosticSeverity, FieldDecl, GeneratedFile, GeneratedFiles,
    InvariantDecl, Protocol, TransitionDecl,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

pub fn export_bundle(protocol: &Protocol, source_yaml: impl Into<String>) -> AikenFlowBundle {
    let diagnostics = protocol.validate();
    let aiken = prefixed_artefacts("contracts", aikenflow_aiken_gen::generate(protocol));
    let lucid = prefixed_artefacts("offchain", aikenflow_lucid_gen::generate(protocol));
    let assurance = prefixed_artefacts("assurance", aikenflow_assurance::generate(protocol));
    let tests = assurance
        .iter()
        .filter(|artefact| artefact.path.starts_with("assurance/tests/"))
        .cloned()
        .collect::<Vec<_>>();
    let audit = assurance
        .iter()
        .filter(|artefact| !artefact.path.starts_with("assurance/tests/"))
        .cloned()
        .collect::<Vec<_>>();

    let invariants = invariant_statuses(protocol);
    let generated_files = aiken.len() + lucid.len() + tests.len() + audit.len();
    let metrics = protocol_metrics(protocol, &diagnostics, &invariants, generated_files);
    let transitions = protocol
        .transitions
        .iter()
        .map(|transition| graph_transition(protocol, transition))
        .collect::<Vec<_>>();

    AikenFlowBundle {
        schema_version: "0.1.0".to_owned(),
        protocol: BundleProtocol {
            name: protocol.name.clone(),
            description: protocol.description.clone(),
            source_yaml: source_yaml.into(),
        },
        graph: BundleGraph {
            states: protocol.states.iter().map(graph_state).collect(),
            transactions: transitions.iter().map(graph_transaction).collect(),
            edges: graph_edges(&transitions),
            transitions,
        },
        diagnostics: diagnostics.iter().map(bundle_diagnostic).collect(),
        invariants,
        metrics,
        findings: generated_findings(&diagnostics, &invariant_statuses(protocol)),
        artefacts: BundleArtefacts {
            aiken,
            lucid,
            tests,
            audit,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AikenFlowBundle {
    pub schema_version: String,
    pub protocol: BundleProtocol,
    pub graph: BundleGraph,
    pub diagnostics: Vec<BundleDiagnostic>,
    pub invariants: Vec<InvariantStatus>,
    pub metrics: ProtocolMetrics,
    pub findings: Vec<GeneratedFinding>,
    pub artefacts: BundleArtefacts,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleProtocol {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source_yaml: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGraph {
    pub states: Vec<GraphState>,
    pub transactions: Vec<GraphTransaction>,
    pub edges: Vec<GraphEdge>,
    /// Compatibility field consumed by the v0.1 webview. The semantic topology
    /// is the same data as `transactions` plus `edges`.
    pub transitions: Vec<GraphTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphState {
    pub id: String,
    pub label: String,
    pub datum_fields: Vec<GraphDatumField>,
    #[serde(skip_serializing_if = "is_false")]
    pub terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphDatumField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphTransition {
    pub id: String,
    pub label: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub constraints: Vec<ConstraintSummary>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphTransaction {
    pub id: String,
    pub label: String,
    pub transition_id: String,
    pub constraints: Vec<ConstraintSummary>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: GraphEdgeKind,
    pub label: String,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphEdgeKind {
    Consumes,
    Produces,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintSummary {
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleDiagnostic {
    pub severity: DiagnosticSeverityName,
    pub code: String,
    pub message: String,
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverityName {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvariantStatus {
    pub name: String,
    pub expression: String,
    pub status: CoverageStatus,
    pub transitions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CoverageStatus {
    Covered,
    Partial,
    Missing,
}

impl fmt::Display for CoverageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoverageStatus::Covered => f.write_str("covered"),
            CoverageStatus::Partial => f.write_str("partial"),
            CoverageStatus::Missing => f.write_str("missing"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolMetrics {
    pub state_coverage: u8,
    pub transition_coverage: u8,
    pub invariant_coverage: u8,
    pub generated_files: usize,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedFinding {
    pub id: String,
    pub severity: RiskLevel,
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleArtefacts {
    pub aiken: Vec<GeneratedArtefact>,
    pub lucid: Vec<GeneratedArtefact>,
    pub tests: Vec<GeneratedArtefact>,
    pub audit: Vec<GeneratedArtefact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedArtefact {
    pub path: String,
    pub language: ArtefactLanguage,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtefactLanguage {
    Aiken,
    TypeScript,
    Markdown,
    Json,
    Toml,
    Mermaid,
    Text,
}

fn graph_state(state: &aikenflow_ir::StateDecl) -> GraphState {
    GraphState {
        id: state.name.clone(),
        label: state.name.clone(),
        datum_fields: state.datum.iter().map(graph_datum_field).collect(),
        terminal: state.terminal,
    }
}

fn graph_datum_field(field: &FieldDecl) -> GraphDatumField {
    GraphDatumField {
        name: field.name.clone(),
        ty: field.ty.clone(),
    }
}

fn graph_transition(protocol: &Protocol, transition: &TransitionDecl) -> GraphTransition {
    GraphTransition {
        id: transition.name.clone(),
        label: transition.name.clone(),
        from: transition
            .inputs
            .iter()
            .map(|input| input.state.clone())
            .collect(),
        to: transition
            .outputs
            .iter()
            .map(|output| output.state.clone())
            .collect(),
        constraints: transition
            .constraints
            .iter()
            .map(constraint_summary)
            .collect(),
        risk_level: transition_risk_level(protocol, transition),
    }
}

fn graph_transaction(transition: &GraphTransition) -> GraphTransaction {
    GraphTransaction {
        id: format!("tx:{}", transition.id),
        label: transition.label.clone(),
        transition_id: transition.id.clone(),
        constraints: transition.constraints.clone(),
        risk_level: transition.risk_level,
    }
}

fn graph_edges(transitions: &[GraphTransition]) -> Vec<GraphEdge> {
    let mut edges = Vec::new();

    for transition in transitions {
        let tx_id = format!("tx:{}", transition.id);
        if transition.from.is_empty() {
            edges.push(GraphEdge {
                id: format!("start->{}", transition.id),
                source: "start".to_owned(),
                target: tx_id.clone(),
                kind: GraphEdgeKind::Consumes,
                label: transition.label.clone(),
                risk_level: transition.risk_level,
            });
        } else {
            for state in &transition.from {
                edges.push(GraphEdge {
                    id: format!("state:{state}->{}", transition.id),
                    source: format!("state:{state}"),
                    target: tx_id.clone(),
                    kind: GraphEdgeKind::Consumes,
                    label: transition.label.clone(),
                    risk_level: transition.risk_level,
                });
            }
        }

        if transition.to.is_empty() {
            edges.push(GraphEdge {
                id: format!("{}->terminal", transition.id),
                source: tx_id.clone(),
                target: "terminal".to_owned(),
                kind: GraphEdgeKind::Produces,
                label: transition.label.clone(),
                risk_level: transition.risk_level,
            });
        } else {
            for state in &transition.to {
                edges.push(GraphEdge {
                    id: format!("{}->state:{state}", transition.id),
                    source: tx_id.clone(),
                    target: format!("state:{state}"),
                    kind: GraphEdgeKind::Produces,
                    label: transition.label.clone(),
                    risk_level: transition.risk_level,
                });
            }
        }
    }

    edges
}

fn constraint_summary(constraint: &Constraint) -> ConstraintSummary {
    ConstraintSummary {
        kind: constraint_kind(constraint).to_owned(),
        label: constraint.label(),
    }
}

fn constraint_kind(constraint: &Constraint) -> &'static str {
    match constraint {
        Constraint::SignedBy(_) => "signed_by",
        Constraint::Before(_) => "before",
        Constraint::After(_) => "after",
        Constraint::Positive(_) => "positive",
        Constraint::ValuePreserved(_) => "value_preserved",
        Constraint::OutputExists(_) => "output_exists",
        Constraint::DatumFieldEquals { .. } => "datum_field_equals",
        Constraint::CustomAiken(_) => "custom_aiken",
        Constraint::Unknown { .. } => "unknown",
    }
}

fn transition_risk_level(protocol: &Protocol, transition: &TransitionDecl) -> RiskLevel {
    if transition.constraints.iter().any(|constraint| {
        matches!(
            constraint,
            Constraint::Unknown { .. }
                | Constraint::CustomAiken(_)
                | Constraint::ValuePreserved(_)
                | Constraint::OutputExists(_)
        )
    }) {
        return RiskLevel::High;
    }

    if transition.constraints.is_empty()
        || transition
            .outputs
            .iter()
            .any(|output| output.value.is_empty() && transition.inputs.is_empty())
        || (transition.inputs.is_empty() && !transition.constraints.is_empty())
        || transition
            .constraints
            .iter()
            .any(|constraint| constraint_requires_manual_review(protocol, transition, constraint))
    {
        return RiskLevel::Medium;
    }

    RiskLevel::Low
}

fn constraint_requires_manual_review(
    protocol: &Protocol,
    transition: &TransitionDecl,
    constraint: &Constraint,
) -> bool {
    match constraint {
        Constraint::SignedBy(expr)
        | Constraint::Before(expr)
        | Constraint::After(expr)
        | Constraint::Positive(expr) => {
            !resolves_to_consumed_datum_field(protocol, transition, expr)
        }
        Constraint::DatumFieldEquals { field, value } => {
            !resolves_to_consumed_datum_field(protocol, transition, field)
                || (looks_like_field_expression(value)
                    && !resolves_to_consumed_datum_field(protocol, transition, value))
        }
        Constraint::ValuePreserved(_)
        | Constraint::OutputExists(_)
        | Constraint::CustomAiken(_)
        | Constraint::Unknown { .. } => true,
    }
}

fn resolves_to_consumed_datum_field(
    protocol: &Protocol,
    transition: &TransitionDecl,
    expr: &str,
) -> bool {
    let candidate = datum_field_candidate(expr);
    transition.inputs.iter().any(|input| {
        protocol
            .state(&input.state)
            .is_some_and(|state| state.datum.iter().any(|field| field.name == candidate))
    })
}

fn datum_field_candidate(expr: &str) -> &str {
    let trimmed = expr.trim().trim_matches('"');
    trimmed
        .strip_prefix("datum.")
        .or_else(|| trimmed.strip_prefix('$'))
        .unwrap_or(trimmed)
}

fn looks_like_field_expression(expr: &str) -> bool {
    let trimmed = expr.trim().trim_matches('"');
    trimmed.starts_with("datum.") || trimmed.starts_with('$')
}

fn bundle_diagnostic(diagnostic: &Diagnostic) -> BundleDiagnostic {
    BundleDiagnostic {
        severity: match diagnostic.severity {
            DiagnosticSeverity::Error => DiagnosticSeverityName::Error,
            DiagnosticSeverity::Warning => DiagnosticSeverityName::Warning,
            DiagnosticSeverity::Info => DiagnosticSeverityName::Info,
        },
        code: diagnostic.code.clone(),
        message: diagnostic.message.clone(),
        hint: diagnostic.hint.clone(),
        target: None,
    }
}

fn generated_findings(
    diagnostics: &[Diagnostic],
    invariants: &[InvariantStatus],
) -> Vec<GeneratedFinding> {
    let mut findings = diagnostics
        .iter()
        .map(|diagnostic| GeneratedFinding {
            id: format!("diagnostic:{}", diagnostic.code),
            severity: match diagnostic.severity {
                DiagnosticSeverity::Error => RiskLevel::High,
                DiagnosticSeverity::Warning => RiskLevel::Medium,
                DiagnosticSeverity::Info => RiskLevel::Low,
            },
            title: format!("{}: {}", diagnostic.code, diagnostic.message),
            detail: diagnostic.hint.clone(),
            transition_id: None,
        })
        .collect::<Vec<_>>();

    for (index, invariant) in invariants.iter().enumerate() {
        if invariant.status == CoverageStatus::Covered {
            continue;
        }
        findings.push(GeneratedFinding {
            id: format!("invariant:{}:{index}", invariant.name),
            severity: match invariant.status {
                CoverageStatus::Covered => RiskLevel::Low,
                CoverageStatus::Partial => RiskLevel::Medium,
                CoverageStatus::Missing => RiskLevel::High,
            },
            title: format!("{} is {}", invariant.name, invariant.status),
            detail: if invariant.transitions.is_empty() {
                "No transition mapping generated by IR assurance.".to_owned()
            } else {
                format!("Mapped to {}.", invariant.transitions.join(", "))
            },
            transition_id: invariant.transitions.first().cloned(),
        });
    }

    findings
}

fn invariant_statuses(protocol: &Protocol) -> Vec<InvariantStatus> {
    protocol
        .invariants
        .iter()
        .map(|invariant| invariant_status(protocol, invariant))
        .collect()
}

fn invariant_status(protocol: &Protocol, invariant: &InvariantDecl) -> InvariantStatus {
    let transitions = transitions_for_invariant(protocol, invariant);
    let has_generated_review_cases = transitions
        .iter()
        .any(|transition| transition_has_generated_review_case(transition));
    let status = if transitions.is_empty() {
        CoverageStatus::Missing
    } else if has_generated_review_cases {
        CoverageStatus::Partial
    } else {
        CoverageStatus::Missing
    };

    InvariantStatus {
        name: invariant.name.clone(),
        expression: invariant.expression.clone(),
        status,
        transitions: transitions
            .iter()
            .map(|transition| transition.name.clone())
            .collect(),
    }
}

fn transitions_for_invariant<'a>(
    protocol: &'a Protocol,
    invariant: &InvariantDecl,
) -> Vec<&'a TransitionDecl> {
    protocol
        .transitions
        .iter()
        .filter(|transition| {
            invariant
                .expression
                .contains(&format!("transition.{}", transition.name))
        })
        .collect()
}

fn transition_has_generated_review_case(transition: &TransitionDecl) -> bool {
    transition.constraints.iter().any(|constraint| {
        matches!(
            constraint,
            Constraint::SignedBy(_)
                | Constraint::After(_)
                | Constraint::Before(_)
                | Constraint::Positive(_)
                | Constraint::DatumFieldEquals { .. }
        )
    })
}

fn protocol_metrics(
    protocol: &Protocol,
    diagnostics: &[Diagnostic],
    invariants: &[InvariantStatus],
    generated_files: usize,
) -> ProtocolMetrics {
    let state_coverage = percentage(touched_states(protocol).len(), protocol.states.len());
    let transition_coverage = percentage(protocol.transitions.len(), protocol.transitions.len());
    let invariant_coverage = invariant_coverage(invariants);

    ProtocolMetrics {
        state_coverage,
        transition_coverage,
        invariant_coverage,
        generated_files,
        risk_level: protocol_risk_level(protocol, diagnostics, invariants),
    }
}

fn touched_states(protocol: &Protocol) -> BTreeSet<String> {
    let mut states = BTreeSet::new();
    for transition in &protocol.transitions {
        for input in transition.inputs.iter().chain(&transition.reference_inputs) {
            states.insert(input.state.clone());
        }
        for output in &transition.outputs {
            states.insert(output.state.clone());
        }
    }
    states
}

fn protocol_risk_level(
    protocol: &Protocol,
    diagnostics: &[Diagnostic],
    invariants: &[InvariantStatus],
) -> RiskLevel {
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        || protocol
            .transitions
            .iter()
            .any(|transition| transition_risk_level(protocol, transition) == RiskLevel::High)
    {
        return RiskLevel::High;
    }

    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
        || invariants
            .iter()
            .any(|invariant| invariant.status != CoverageStatus::Covered)
        || protocol
            .transitions
            .iter()
            .any(|transition| transition_risk_level(protocol, transition) == RiskLevel::Medium)
    {
        return RiskLevel::Medium;
    }

    RiskLevel::Low
}

fn percentage(numerator: usize, denominator: usize) -> u8 {
    if denominator == 0 {
        return 0;
    }

    ((numerator * 100) / denominator).min(100) as u8
}

fn invariant_coverage(invariants: &[InvariantStatus]) -> u8 {
    if invariants.is_empty() {
        return 0;
    }

    let score = invariants
        .iter()
        .map(|invariant| match invariant.status {
            CoverageStatus::Covered => 100usize,
            CoverageStatus::Partial => 50usize,
            CoverageStatus::Missing => 0usize,
        })
        .sum::<usize>();

    (score / invariants.len()).min(100) as u8
}

fn prefixed_artefacts(prefix: &str, files: GeneratedFiles) -> Vec<GeneratedArtefact> {
    files
        .files
        .into_iter()
        .map(|file| generated_artefact(prefix, file))
        .collect()
}

fn generated_artefact(prefix: &str, file: GeneratedFile) -> GeneratedArtefact {
    let path = format!("{prefix}/{}", file.path);
    GeneratedArtefact {
        language: artefact_language(&path),
        path,
        content: file.content,
    }
}

fn artefact_language(path: &str) -> ArtefactLanguage {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
    {
        "ak" => ArtefactLanguage::Aiken,
        "ts" | "tsx" => ArtefactLanguage::TypeScript,
        "md" => ArtefactLanguage::Markdown,
        "json" => ArtefactLanguage::Json,
        "toml" => ArtefactLanguage::Toml,
        "mmd" => ArtefactLanguage::Mermaid,
        _ => ArtefactLanguage::Text,
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;

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
  - name: global_conservation
    expression: "total value is preserved"
"#;

    #[test]
    fn exports_frontend_bundle_shape() {
        let protocol = aikenflow_parser::parse_protocol(SIMPLE_VAULT).expect("protocol");

        let bundle = export_bundle(&protocol, SIMPLE_VAULT);

        assert_eq!(bundle.schema_version, "0.1.0");
        assert_eq!(bundle.protocol.name, "SimpleVault");
        assert!(bundle
            .protocol
            .source_yaml
            .contains("protocol: SimpleVault"));
        assert_eq!(bundle.graph.states.len(), 1);
        assert_eq!(bundle.graph.transactions.len(), 2);
        assert!(!bundle.graph.edges.is_empty());
        assert_eq!(bundle.graph.transitions.len(), 2);
        assert!(bundle
            .artefacts
            .aiken
            .iter()
            .any(|artefact| artefact.path.ends_with(".ak")));
        assert!(bundle
            .artefacts
            .lucid
            .iter()
            .any(|artefact| artefact.path == "offchain/src/index.ts"));
        assert!(bundle
            .artefacts
            .tests
            .iter()
            .any(|artefact| artefact.path == "assurance/tests/adversarial.spec.ts"));
        assert!(bundle
            .artefacts
            .audit
            .iter()
            .any(|artefact| artefact.path == "assurance/AUDIT.md"));
        assert!(bundle.metrics.generated_files > 0);
        assert!(!bundle.findings.is_empty());
    }

    #[test]
    fn invariants_are_not_exported_as_formally_covered() {
        let protocol = aikenflow_parser::parse_protocol(SIMPLE_VAULT).expect("protocol");

        let bundle = export_bundle(&protocol, SIMPLE_VAULT);

        assert_eq!(bundle.invariants[0].status, CoverageStatus::Partial);
        assert_eq!(bundle.invariants[1].status, CoverageStatus::Missing);
        assert_eq!(bundle.metrics.invariant_coverage, 25);
        assert_eq!(bundle.metrics.risk_level, RiskLevel::Medium);
        assert!(bundle
            .invariants
            .iter()
            .all(|invariant| invariant.status != CoverageStatus::Covered));
    }

    #[test]
    fn marks_unenforced_parameter_constraints_medium() {
        let protocol = aikenflow_parser::parse_protocol(SIMPLE_VAULT).expect("protocol");

        let bundle = export_bundle(&protocol, SIMPLE_VAULT);
        let deposit = bundle
            .graph
            .transitions
            .iter()
            .find(|transition| transition.id == "Deposit")
            .expect("deposit transition");
        let withdraw = bundle
            .graph
            .transitions
            .iter()
            .find(|transition| transition.id == "Withdraw")
            .expect("withdraw transition");

        assert_eq!(deposit.risk_level, RiskLevel::Medium);
        assert_eq!(withdraw.risk_level, RiskLevel::Low);
    }

    #[test]
    fn serializes_with_frontend_camel_case_fields() {
        let protocol = aikenflow_parser::parse_protocol(SIMPLE_VAULT).expect("protocol");
        let bundle = export_bundle(&protocol, SIMPLE_VAULT);

        let json = serde_json::to_string(&bundle).expect("json");

        assert!(json.contains("\"schemaVersion\""));
        assert!(json.contains("\"sourceYaml\""));
        assert!(json.contains("\"riskLevel\""));
        assert!(json.contains("\"datumFields\""));
    }

    #[test]
    fn marks_unsupported_transition_risk_high() {
        let mut protocol = aikenflow_parser::parse_protocol(SIMPLE_VAULT).expect("protocol");
        protocol.transitions[0]
            .constraints
            .push(Constraint::ValuePreserved(None));

        let bundle = export_bundle(&protocol, SIMPLE_VAULT);

        assert_eq!(bundle.graph.transitions[0].risk_level, RiskLevel::High);
        assert_eq!(bundle.metrics.risk_level, RiskLevel::High);
    }

    #[test]
    fn classifies_generated_artefact_languages() {
        assert_eq!(
            artefact_language("contracts/aiken.toml"),
            ArtefactLanguage::Toml
        );
        assert_eq!(
            artefact_language("contracts/validators/vault.ak"),
            ArtefactLanguage::Aiken
        );
        assert_eq!(
            artefact_language("offchain/src/index.ts"),
            ArtefactLanguage::TypeScript
        );
        assert_eq!(
            artefact_language("assurance/state-graph.mmd"),
            ArtefactLanguage::Mermaid
        );
    }
}
