import type { AikenFlowDiagnostic, ProtocolSelection, SourceRange } from "./aikenflowTypes";

export function findSourceRangeForSelection(source: string, selection: ProtocolSelection): SourceRange | undefined {
  switch (selection.kind) {
    case "state":
      return findNamedMapEntry(source, "states", selection.id);
    case "transition":
      return findNamedMapEntry(source, "transitions", selection.id);
    case "invariant":
      return findNamedListEntry(source, "invariants", selection.id);
    case "diagnostic":
    case "artefact":
      return undefined;
  }
}

export function attachSourceRangesToDiagnostics(
  source: string,
  diagnostics: AikenFlowDiagnostic[],
): AikenFlowDiagnostic[] {
  return diagnostics.map((diagnostic) => ({
    ...diagnostic,
    range: diagnostic.range ?? findSourceRangeForDiagnostic(source, diagnostic),
  }));
}

export function findSourceRangeForDiagnostic(
  source: string,
  diagnostic: Pick<AikenFlowDiagnostic, "message" | "target">,
): SourceRange | undefined {
  const targetSelection = selectionFromTarget(diagnostic.target);
  if (targetSelection) {
    const range = findSourceRangeForSelection(source, targetSelection);
    if (range) return range;
  }

  const transition = firstBacktickedNameAfter(diagnostic.message, "Transition");
  if (transition) {
    const range = findSourceRangeForSelection(source, { kind: "transition", id: transition });
    if (range) return range;
  }

  const state = firstBacktickedNameAfter(diagnostic.message, "State");
  if (state) {
    const range = findSourceRangeForSelection(source, { kind: "state", id: state });
    if (range) return range;
  }

  const invariant = firstBacktickedNameAfter(diagnostic.message, "Invariant");
  if (invariant) {
    const range = findSourceRangeForSelection(source, { kind: "invariant", id: invariant });
    if (range) return range;
  }

  return findTopLevelKey(source, "protocol");
}

function findNamedMapEntry(source: string, section: string, name: string): SourceRange | undefined {
  const lines = source.split(/\r?\n/);
  const sectionLine = findTopLevelSection(lines, section);
  if (sectionLine === undefined) return undefined;

  for (let index = sectionLine + 1; index < lines.length; index += 1) {
    const line = lines[index] ?? "";
    if (isTopLevelSection(line)) break;
    const match = /^(\s+)([^#:\s][^:]*):\s*(?:#.*)?$/.exec(line);
    if (!match) continue;
    if ((match[1]?.length ?? 0) <= 0) continue;
    if (unquote(match[2]?.trim() ?? "") === name) {
      return lineRange(index, line);
    }
  }

  return undefined;
}

function findNamedListEntry(source: string, section: string, name: string): SourceRange | undefined {
  const lines = source.split(/\r?\n/);
  const sectionLine = findTopLevelSection(lines, section);
  if (sectionLine === undefined) return undefined;

  for (let index = sectionLine + 1; index < lines.length; index += 1) {
    const line = lines[index] ?? "";
    if (isTopLevelSection(line)) break;
    const match = /^\s*-\s+name:\s*(.+?)\s*(?:#.*)?$/.exec(line);
    if (!match) continue;
    if (unquote(match[1]?.trim() ?? "") === name) {
      return lineRange(index, line);
    }
  }

  return undefined;
}

function findTopLevelSection(lines: string[], section: string): number | undefined {
  const pattern = new RegExp(`^${escapeRegExp(section)}:\\s*(?:#.*)?$`);
  const index = lines.findIndex((line) => pattern.test(line));
  return index >= 0 ? index : undefined;
}

function findTopLevelKey(source: string, key: string): SourceRange | undefined {
  const lines = source.split(/\r?\n/);
  const pattern = new RegExp(`^${escapeRegExp(key)}:\\s*`);
  const index = lines.findIndex((line) => pattern.test(line));
  if (index < 0) return undefined;
  return lineRange(index, lines[index] ?? "");
}

function isTopLevelSection(line: string): boolean {
  return /^[A-Za-z_][A-Za-z0-9_-]*:\s*(?:#.*)?$/.test(line);
}

function lineRange(index: number, line: string): SourceRange {
  return {
    startLine: index + 1,
    startColumn: Math.max(1, line.search(/\S/) + 1),
    endLine: index + 1,
    endColumn: line.length + 1,
  };
}

function unquote(value: string): string {
  if (
    (value.startsWith("\"") && value.endsWith("\"")) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    return value.slice(1, -1);
  }
  return value;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function firstBacktickedNameAfter(message: string, label: string): string | undefined {
  const match = new RegExp(`${escapeRegExp(label)}\\s+\`([^\`]+)\``, "i").exec(message);
  return match?.[1];
}

function selectionFromTarget(target: AikenFlowDiagnostic["target"]): ProtocolSelection | undefined {
  if (!target) return undefined;
  if (typeof target === "object") {
    if (!target.kind || !target.name) return undefined;
    switch (target.kind) {
      case "state":
        return { kind: "state", id: target.name };
      case "transition":
      case "transaction":
        return { kind: "transition", id: target.name };
      case "invariant":
        return { kind: "invariant", id: target.name };
      default:
        return undefined;
    }
  }
  const [kind, ...rest] = target.split(":");
  const id = rest.join(":");
  if (!id) return undefined;
  switch (kind) {
    case "state":
      return { kind: "state", id };
    case "transition":
      return { kind: "transition", id };
    case "invariant":
      return { kind: "invariant", id };
    default:
      return undefined;
  }
}
