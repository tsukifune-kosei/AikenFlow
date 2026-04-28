import test from "node:test";
import assert from "node:assert/strict";
import { detectManifestConflicts, sha256 } from "./generatedManifest";

test("detectManifestConflicts reports modified generated files", () => {
  const previous = {
    files: [
      {
        path: "contracts/validator.ak",
        contentHash: sha256("old generated content"),
      },
    ],
  };
  const conflicts = detectManifestConflicts(
    previous,
    new Map([["contracts/validator.ak", "manual edit"]]),
  );
  assert.equal(conflicts.length, 1);
  assert.equal(conflicts[0]?.path, "contracts/validator.ak");
});

test("detectManifestConflicts ignores unchanged files", () => {
  const content = "generated content";
  const previous = {
    files: [
      {
        path: "contracts/validator.ak",
        contentHash: sha256(content),
      },
    ],
  };
  const conflicts = detectManifestConflicts(
    previous,
    new Map([["contracts/validator.ak", content]]),
  );
  assert.deepEqual(conflicts, []);
});
