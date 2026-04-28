import * as vscode from "vscode";
import type { AikenFlowDiagnostic, SourceRange } from "../shared/aikenflowTypes";

export function toVsCodeDiagnostic(diagnostic: AikenFlowDiagnostic): vscode.Diagnostic | undefined {
  if (!diagnostic.range) {
    return undefined;
  }

  const mapped = new vscode.Diagnostic(
    toRange(diagnostic.range),
    `${diagnostic.code ? `${diagnostic.code}: ` : ""}${diagnostic.message}${diagnostic.hint ? `\n${diagnostic.hint}` : ""}`,
    toSeverity(diagnostic.severity),
  );
  mapped.source = "AikenFlow";
  mapped.code = diagnostic.code;
  return mapped;
}

function toRange(range: SourceRange): vscode.Range {
  return new vscode.Range(
    Math.max(0, range.startLine - 1),
    Math.max(0, range.startColumn - 1),
    Math.max(0, range.endLine - 1),
    Math.max(0, range.endColumn - 1),
  );
}

function toSeverity(severity: AikenFlowDiagnostic["severity"]): vscode.DiagnosticSeverity {
  switch (severity) {
    case "error":
      return vscode.DiagnosticSeverity.Error;
    case "warning":
      return vscode.DiagnosticSeverity.Warning;
    case "info":
      return vscode.DiagnosticSeverity.Information;
  }
}
