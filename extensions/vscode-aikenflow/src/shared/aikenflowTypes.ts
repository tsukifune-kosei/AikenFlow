export type RiskLevel = "low" | "medium" | "high";
export type CoverageStatus = "covered" | "partial" | "missing";
export type DiagnosticSeverity = "info" | "warning" | "error";
export type ArtefactLanguage =
  | "aiken"
  | "typescript"
  | "markdown"
  | "json"
  | "toml"
  | "mermaid"
  | "text";

export type AikenFlowBundle = {
  schemaVersion: string;
  protocol: {
    name: string;
    description?: string;
    sourceYaml: string;
  };
  graph: {
    states: GraphState[];
    transactions: GraphTransaction[];
    edges: GraphEdge[];
    transitions: GraphTransition[];
  };
  diagnostics: AikenFlowDiagnostic[];
  invariants: InvariantStatus[];
  metrics: ProtocolMetrics;
  findings: GeneratedFinding[];
  artefacts: {
    aiken: GeneratedArtefact[];
    lucid: GeneratedArtefact[];
    tests: GeneratedArtefact[];
    audit: GeneratedArtefact[];
  };
};

export type GraphState = {
  id: string;
  label: string;
  datumFields: DatumField[];
  terminal?: boolean;
};

export type DatumField = {
  name: string;
  type: string;
};

export type GraphTransition = {
  id: string;
  label: string;
  from: string[];
  to: string[];
  constraints: ConstraintSummary[];
  riskLevel: RiskLevel;
};

export type GraphTransaction = {
  id: string;
  label: string;
  transitionId: string;
  constraints: ConstraintSummary[];
  riskLevel: RiskLevel;
};

export type GraphEdge = {
  id: string;
  source: string;
  target: string;
  kind: "consumes" | "produces";
  label: string;
  riskLevel: RiskLevel;
};

export type ConstraintSummary = {
  kind: string;
  label: string;
};

export type SourceRange = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type AikenFlowDiagnostic = {
  severity: DiagnosticSeverity;
  code?: string;
  message: string;
  hint?: string;
  target?: string | DiagnosticTarget;
  range?: SourceRange;
};

export type DiagnosticTarget = {
  kind?: string | null;
  name?: string | null;
  field?: string | null;
  line?: number | null;
  column?: number | null;
};

export type InvariantStatus = {
  name: string;
  expression: string;
  status: CoverageStatus;
  transitions: string[];
};

export type ProtocolMetrics = {
  stateCoverage: number;
  transitionCoverage: number;
  invariantCoverage: number;
  generatedFiles: number;
  riskLevel: RiskLevel;
};

export type GeneratedFinding = {
  id: string;
  severity: RiskLevel;
  title: string;
  detail: string;
  transitionId?: string;
};

export type GeneratedArtefact = {
  path: string;
  language: ArtefactLanguage;
  content: string;
};

export type ArtefactGroup = keyof AikenFlowBundle["artefacts"];

export type ProtocolSelection =
  | { kind: "state"; id: string }
  | { kind: "transition"; id: string }
  | { kind: "invariant"; id: string }
  | { kind: "diagnostic"; id: string }
  | { kind: "artefact"; path: string };

export function isAikenFlowBundle(value: unknown): value is AikenFlowBundle {
  if (!isRecord(value)) return false;
  const maybe = value as Partial<AikenFlowBundle>;
  return (
    typeof maybe.schemaVersion === "string" &&
    typeof maybe.protocol?.name === "string" &&
    typeof maybe.protocol?.sourceYaml === "string" &&
    Array.isArray(maybe.graph?.states) &&
    maybe.graph.states.every(isGraphState) &&
    Array.isArray(maybe.graph?.transactions) &&
    maybe.graph.transactions.every(isGraphTransaction) &&
    Array.isArray(maybe.graph?.edges) &&
    maybe.graph.edges.every(isGraphEdge) &&
    Array.isArray(maybe.graph?.transitions) &&
    maybe.graph.transitions.every(isGraphTransition) &&
    Array.isArray(maybe.diagnostics) &&
    maybe.diagnostics.every(isAikenFlowDiagnostic) &&
    Array.isArray(maybe.invariants) &&
    maybe.invariants.every(isInvariantStatus) &&
    typeof maybe.metrics?.stateCoverage === "number" &&
    typeof maybe.metrics.transitionCoverage === "number" &&
    typeof maybe.metrics.invariantCoverage === "number" &&
    typeof maybe.metrics?.generatedFiles === "number" &&
    isRiskLevel(maybe.metrics.riskLevel) &&
    Array.isArray(maybe.findings) &&
    maybe.findings.every(isGeneratedFinding) &&
    isArtefactGroups(maybe.artefacts)
  );
}

function isGraphState(value: unknown): value is GraphState {
  return isRecord(value)
    && typeof value.id === "string"
    && typeof value.label === "string"
    && Array.isArray(value.datumFields)
    && value.datumFields.every(isDatumField)
    && (value.terminal === undefined || typeof value.terminal === "boolean");
}

function isDatumField(value: unknown): value is DatumField {
  return isRecord(value) && typeof value.name === "string" && typeof value.type === "string";
}

function isGraphTransition(value: unknown): value is GraphTransition {
  return isRecord(value)
    && typeof value.id === "string"
    && typeof value.label === "string"
    && Array.isArray(value.from)
    && value.from.every((item) => typeof item === "string")
    && Array.isArray(value.to)
    && value.to.every((item) => typeof item === "string")
    && Array.isArray(value.constraints)
    && value.constraints.every(isConstraintSummary)
    && isRiskLevel(value.riskLevel);
}

function isGraphTransaction(value: unknown): value is GraphTransaction {
  return isRecord(value)
    && typeof value.id === "string"
    && typeof value.label === "string"
    && typeof value.transitionId === "string"
    && Array.isArray(value.constraints)
    && value.constraints.every(isConstraintSummary)
    && isRiskLevel(value.riskLevel);
}

function isGraphEdge(value: unknown): value is GraphEdge {
  return isRecord(value)
    && typeof value.id === "string"
    && typeof value.source === "string"
    && typeof value.target === "string"
    && (value.kind === "consumes" || value.kind === "produces")
    && typeof value.label === "string"
    && isRiskLevel(value.riskLevel);
}

function isConstraintSummary(value: unknown): value is ConstraintSummary {
  return isRecord(value) && typeof value.kind === "string" && typeof value.label === "string";
}

function isAikenFlowDiagnostic(value: unknown): value is AikenFlowDiagnostic {
  return isRecord(value)
    && isDiagnosticSeverity(value.severity)
    && typeof value.message === "string"
    && (value.code === undefined || typeof value.code === "string")
    && (value.hint === undefined || typeof value.hint === "string")
    && (value.target === undefined || typeof value.target === "string" || isDiagnosticTarget(value.target))
    && (value.range === undefined || isSourceRange(value.range));
}

function isDiagnosticTarget(value: unknown): value is DiagnosticTarget {
  return isRecord(value)
    && (value.kind === undefined || value.kind === null || typeof value.kind === "string")
    && (value.name === undefined || value.name === null || typeof value.name === "string")
    && (value.field === undefined || value.field === null || typeof value.field === "string")
    && (value.line === undefined || value.line === null || isPositiveInteger(value.line))
    && (value.column === undefined || value.column === null || isPositiveInteger(value.column));
}

function isSourceRange(value: unknown): value is SourceRange {
  return isRecord(value)
    && isPositiveInteger(value.startLine)
    && isPositiveInteger(value.startColumn)
    && isPositiveInteger(value.endLine)
    && isPositiveInteger(value.endColumn);
}

function isInvariantStatus(value: unknown): value is InvariantStatus {
  return isRecord(value)
    && typeof value.name === "string"
    && typeof value.expression === "string"
    && isCoverageStatus(value.status)
    && Array.isArray(value.transitions)
    && value.transitions.every((item) => typeof item === "string");
}

function isArtefactGroups(value: unknown): value is AikenFlowBundle["artefacts"] {
  if (!isRecord(value)) return false;
  return ["aiken", "lucid", "tests", "audit"].every((group) => {
    const entries = value[group];
    return Array.isArray(entries) && entries.every(isGeneratedArtefact);
  });
}

function isGeneratedArtefact(value: unknown): value is GeneratedArtefact {
  return isRecord(value)
    && typeof value.path === "string"
    && isSafeRelativePath(value.path)
    && isArtefactLanguage(value.language)
    && typeof value.content === "string";
}

function isGeneratedFinding(value: unknown): value is GeneratedFinding {
  return isRecord(value)
    && typeof value.id === "string"
    && isRiskLevel(value.severity)
    && typeof value.title === "string"
    && typeof value.detail === "string"
    && (value.transitionId === undefined || typeof value.transitionId === "string");
}

function isRiskLevel(value: unknown): value is RiskLevel {
  return value === "low" || value === "medium" || value === "high";
}

function isCoverageStatus(value: unknown): value is CoverageStatus {
  return value === "covered" || value === "partial" || value === "missing";
}

function isDiagnosticSeverity(value: unknown): value is DiagnosticSeverity {
  return value === "info" || value === "warning" || value === "error";
}

function isArtefactLanguage(value: unknown): value is ArtefactLanguage {
  return value === "aiken"
    || value === "typescript"
    || value === "markdown"
    || value === "json"
    || value === "toml"
    || value === "mermaid"
    || value === "text";
}

function isPositiveInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value > 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object";
}

function isSafeRelativePath(value: string): boolean {
  if (!value || value.startsWith("/") || value.startsWith("\\") || /^[a-zA-Z]:/.test(value)) {
    return false;
  }
  const parts = value.split(/[\\/]+/);
  return parts.every((part) => part.length > 0 && part !== "." && part !== ".." && !part.includes("\0"));
}
