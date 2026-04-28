import { createHash } from "node:crypto";

export type ManifestFile = {
  path: string;
  contentHash: string;
};

export type GeneratedManifest = {
  files: ManifestFile[];
};

export type ManifestConflict = {
  path: string;
  expectedHash: string;
  actualHash: string;
};

export function sha256(content: string | Buffer): string {
  return createHash("sha256").update(content).digest("hex");
}

export function detectManifestConflicts(
  previous: GeneratedManifest,
  currentFiles: Map<string, string | Buffer>,
): ManifestConflict[] {
  const conflicts: ManifestConflict[] = [];
  for (const file of previous.files) {
    const currentContent = currentFiles.get(file.path);
    if (currentContent === undefined) continue;
    const actualHash = sha256(currentContent);
    if (actualHash !== file.contentHash) {
      conflicts.push({
        path: file.path,
        expectedHash: file.contentHash,
        actualHash,
      });
    }
  }
  return conflicts;
}
