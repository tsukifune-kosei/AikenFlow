import test from "node:test";
import assert from "node:assert/strict";
import type { GeneratedArtefact } from "./aikenflowTypes";
import { findArtefactLineForSelection } from "./artefactLocator";

const artefact: GeneratedArtefact = {
  path: "offchain/src/index.ts",
  language: "typescript",
  content: `export interface DepositParams {}

export async function deposit(params: DepositParams) {
  return params;
}

export interface EmergencyCancelParams {}

export async function emergencyCancel(params: EmergencyCancelParams) {
  return params;
}
`,
};

test("findArtefactLineForSelection locates exact transition names", () => {
  const line = findArtefactLineForSelection(artefact, { kind: "transition", id: "Deposit" });
  assert.equal(line, 1);
});

test("findArtefactLineForSelection locates camel-case generated functions", () => {
  const line = findArtefactLineForSelection(artefact, { kind: "transition", id: "EmergencyCancel" });
  assert.equal(line, 7);
});

test("findArtefactLineForSelection returns undefined without a matching symbol", () => {
  const line = findArtefactLineForSelection(artefact, { kind: "transition", id: "Withdraw" });
  assert.equal(line, undefined);
});
