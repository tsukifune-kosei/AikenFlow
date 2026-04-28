import assert from "node:assert/strict";
import test from "node:test";
import { safeRelativePathParts } from "./pathSafety";

test("safeRelativePathParts accepts simple workspace-relative paths", () => {
  assert.deepEqual(safeRelativePathParts(".aikenflow/bundle.json", "bundlePath"), [
    ".aikenflow",
    "bundle.json",
  ]);
  assert.deepEqual(safeRelativePathParts("generated", "generatedOutputDir"), ["generated"]);
});

test("safeRelativePathParts rejects parent traversal", () => {
  assert.throws(
    () => safeRelativePathParts("../outside/bundle.json", "bundlePath"),
    /must not contain/,
  );
  assert.throws(
    () => safeRelativePathParts("generated/../../outside", "generatedOutputDir"),
    /must not contain/,
  );
});

test("safeRelativePathParts rejects absolute paths", () => {
  assert.throws(
    () => safeRelativePathParts("/tmp/aikenflow/bundle.json", "bundlePath"),
    /workspace-relative/,
  );
  assert.throws(
    () => safeRelativePathParts("C:\\tmp\\aikenflow\\bundle.json", "bundlePath"),
    /workspace-relative/,
  );
});

test("safeRelativePathParts rejects empty and current-directory segments", () => {
  assert.throws(() => safeRelativePathParts("", "bundlePath"), /must not be empty/);
  assert.throws(() => safeRelativePathParts("./bundle.json", "bundlePath"), /must not contain/);
  assert.throws(() => safeRelativePathParts("generated//contracts", "generatedOutputDir"), /must not contain/);
});
