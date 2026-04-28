import type { AikenFlowBundle, ProtocolSelection } from "./aikenflowTypes";

export type ExtensionToWebview =
  | { type: "analysis.started"; protocolUri: string }
  | { type: "analysis.completed"; bundle: AikenFlowBundle; sourceUri: string; elapsedMs: number }
  | { type: "analysis.failed"; message: string; stderr?: string }
  | { type: "bundle.loaded"; bundle: AikenFlowBundle; sourceUri: string }
  | { type: "selection.changed"; selection: ProtocolSelection };

export type WebviewToExtension =
  | { type: "command.analyse" }
  | { type: "command.generateArtefacts" }
  | { type: "command.exportAudit" }
  | { type: "reveal.source"; target: ProtocolSelection }
  | { type: "reveal.artefact"; path: string; line?: number }
  | { type: "selection.changed"; selection: ProtocolSelection };

export function isWebviewToExtensionMessage(value: unknown): value is WebviewToExtension {
  if (!isRecord(value)) return false;
  switch (value.type) {
    case "command.analyse":
    case "command.generateArtefacts":
    case "command.exportAudit":
      return true;
    case "reveal.source":
      return isProtocolSelection(value.target);
    case "reveal.artefact":
      return typeof value.path === "string"
        && isSafeRelativePath(value.path)
        && (value.line === undefined || isPositiveInteger(value.line));
    case "selection.changed":
      return isProtocolSelection(value.selection);
    default:
      return false;
  }
}

function isProtocolSelection(value: unknown): value is ProtocolSelection {
  if (!isRecord(value)) return false;
  switch (value.kind) {
    case "state":
    case "transition":
    case "invariant":
    case "diagnostic":
      return typeof value.id === "string" && value.id.length > 0;
    case "artefact":
      return typeof value.path === "string" && isSafeRelativePath(value.path);
    default:
      return false;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object";
}

function isPositiveInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value > 0;
}

function isSafeRelativePath(value: string): boolean {
  if (!value || value.startsWith("/") || value.startsWith("\\") || /^[a-zA-Z]:/.test(value)) {
    return false;
  }
  const parts = value.split(/[\\/]+/);
  return parts.every((part) => part.length > 0 && part !== "." && part !== ".." && !part.includes("\0"));
}
