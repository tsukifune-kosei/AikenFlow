import assert from "node:assert/strict";
import test from "node:test";
import { isWebviewToExtensionMessage } from "./webviewMessages";

test("isWebviewToExtensionMessage accepts command messages", () => {
  assert.equal(isWebviewToExtensionMessage({ type: "command.analyse" }), true);
  assert.equal(isWebviewToExtensionMessage({ type: "command.generateArtefacts" }), true);
  assert.equal(isWebviewToExtensionMessage({ type: "command.exportAudit" }), true);
});

test("isWebviewToExtensionMessage validates source selections", () => {
  assert.equal(
    isWebviewToExtensionMessage({
      type: "reveal.source",
      target: { kind: "transition", id: "Withdraw" },
    }),
    true,
  );
  assert.equal(isWebviewToExtensionMessage({ type: "reveal.source" }), false);
  assert.equal(
    isWebviewToExtensionMessage({
      type: "reveal.source",
      target: { kind: "transition", id: "" },
    }),
    false,
  );
});

test("isWebviewToExtensionMessage validates artefact reveal payloads", () => {
  assert.equal(
    isWebviewToExtensionMessage({
      type: "reveal.artefact",
      path: "contracts/validators/vault.ak",
      line: 12,
    }),
    true,
  );
  assert.equal(isWebviewToExtensionMessage({ type: "reveal.artefact", path: "" }), false);
  assert.equal(
    isWebviewToExtensionMessage({
      type: "reveal.artefact",
      path: "contracts/validators/vault.ak",
      line: 0,
    }),
    false,
  );
  assert.equal(
    isWebviewToExtensionMessage({
      type: "reveal.artefact",
      path: "../protocol.yaml",
      line: 1,
    }),
    false,
  );
  assert.equal(
    isWebviewToExtensionMessage({
      type: "reveal.artefact",
      path: "/tmp/protocol.yaml",
      line: 1,
    }),
    false,
  );
});

test("isWebviewToExtensionMessage rejects unknown or malformed messages", () => {
  assert.equal(isWebviewToExtensionMessage(undefined), false);
  assert.equal(isWebviewToExtensionMessage({}), false);
  assert.equal(isWebviewToExtensionMessage({ type: "selection.changed" }), false);
  assert.equal(
    isWebviewToExtensionMessage({
      type: "selection.changed",
      selection: { kind: "artefact", path: "offchain/src/index.ts" },
    }),
    true,
  );
});
