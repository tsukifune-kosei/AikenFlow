import type { GeneratedArtefact, ProtocolSelection } from "./aikenflowTypes";

export function findArtefactLineForSelection(
  artefact: GeneratedArtefact,
  selection: ProtocolSelection | undefined,
): number | undefined {
  if (!selection) return undefined;

  const needles = selectionNeedles(selection);
  if (needles.length === 0) return undefined;

  const lines = artefact.content.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const normalizedLine = normalizeSearchText(lines[index] ?? "");
    if (needles.some((needle) => normalizedLine.includes(needle))) {
      return index + 1;
    }
  }

  return undefined;
}

function selectionNeedles(selection: ProtocolSelection): string[] {
  switch (selection.kind) {
    case "state":
    case "transition":
    case "invariant":
      return nameNeedles(selection.id);
    case "artefact":
      return [normalizeSearchText(selection.path)];
    case "diagnostic":
      return [normalizeSearchText(selection.id)];
  }
}

function nameNeedles(name: string): string[] {
  const words = name
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean);
  const snake = words.map((word) => word.toLowerCase()).join("_");
  const kebab = words.map((word) => word.toLowerCase()).join("-");
  const camel = words
    .map((word, index) => {
      const lower = word.toLowerCase();
      return index === 0 ? lower : `${lower.slice(0, 1).toUpperCase()}${lower.slice(1)}`;
    })
    .join("");

  return unique([name, snake, kebab, camel].map(normalizeSearchText).filter(Boolean));
}

function normalizeSearchText(value: string): string {
  return value.toLowerCase();
}

function unique(values: string[]): string[] {
  return [...new Set(values)];
}
