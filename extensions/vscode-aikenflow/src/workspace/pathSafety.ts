import * as path from "node:path";

export function safeRelativePathParts(relativePath: string, label: string): string[] {
  const trimmed = relativePath.trim();
  if (!trimmed) {
    throw new Error(`AikenFlow ${label} must not be empty.`);
  }
  if (path.isAbsolute(trimmed) || /^[a-zA-Z]:[\\/]/.test(trimmed)) {
    throw new Error(`AikenFlow ${label} must be a workspace-relative path: ${relativePath}`);
  }

  const parts = trimmed.split(/[\\/]/);
  if (
    parts.some((part) => (
      part.length === 0 ||
      part === "." ||
      part === ".." ||
      part.includes("\0")
    ))
  ) {
    throw new Error(`AikenFlow ${label} must not contain empty, current, parent, or NUL path segments: ${relativePath}`);
  }

  return parts;
}
