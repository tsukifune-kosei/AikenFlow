import type { AikenFlowDiagnostic } from "../shared/aikenflowTypes";

export type CliRunResult = {
  command: string;
  args: string[];
  cwd: string;
  exitCode: number | null;
  stdout: string;
  stderr: string;
  elapsedMs: number;
  timedOut: boolean;
};

export function parseDiagnosticsJson(stdout: string): AikenFlowDiagnostic[] {
  const trimmed = stdout.trim();
  if (!trimmed) return [];
  const parsed = JSON.parse(trimmed) as unknown;
  if (Array.isArray(parsed)) {
    return parsed.map(parseDiagnostic);
  }
  if (isRecord(parsed) && Array.isArray(parsed.diagnostics)) {
    return parsed.diagnostics.map(parseDiagnostic);
  }
  throw new Error("AikenFlow check --json did not return a diagnostics object.");
}

function parseDiagnostic(value: unknown): AikenFlowDiagnostic {
  if (!value || typeof value !== "object") {
    throw new Error("Invalid AikenFlow diagnostic entry.");
  }
  const item = value as Record<string, unknown>;
  const severity = parseSeverity(item.severity);
  const message = item.message;
  if (typeof message !== "string") {
    throw new Error("Invalid AikenFlow diagnostic message.");
  }
  return {
    severity,
    message,
    code: typeof item.code === "string" ? item.code : undefined,
    hint: typeof item.hint === "string" ? item.hint : undefined,
    target: parseTarget(item.target),
  };
}

function parseTarget(value: unknown): AikenFlowDiagnostic["target"] {
  if (typeof value === "string") {
    return value;
  }
  if (!isRecord(value)) {
    return undefined;
  }
  return {
    kind: typeof value.kind === "string" ? value.kind : null,
    name: typeof value.name === "string" ? value.name : null,
    field: typeof value.field === "string" ? value.field : null,
    line: typeof value.line === "number" && Number.isInteger(value.line) ? value.line : null,
    column: typeof value.column === "number" && Number.isInteger(value.column) ? value.column : null,
  };
}

function parseSeverity(value: unknown): AikenFlowDiagnostic["severity"] {
  if (typeof value !== "string") {
    throw new Error("Invalid AikenFlow diagnostic severity.");
  }
  const normalized = value.toLowerCase();
  if (normalized !== "info" && normalized !== "warning" && normalized !== "error") {
    throw new Error("Invalid AikenFlow diagnostic severity.");
  }
  return normalized;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object";
}
