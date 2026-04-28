import test from "node:test";
import assert from "node:assert/strict";
import { attachSourceRangesToDiagnostics, findSourceRangeForDiagnostic, findSourceRangeForSelection } from "./sourceLocator";

const source = `protocol: Sample

states:
  Locked:
    datum:
      owner: PubKeyHash
  "Quoted State":
    datum: {}

transitions:
  Deposit:
    inputs: []
  Withdraw:
    inputs:
      - state: Locked

invariants:
  - name: withdrawal_requires_owner
    expression: "transition.Withdraw requires signature(datum.owner)"
`;

test("findSourceRangeForSelection locates states", () => {
  const range = findSourceRangeForSelection(source, { kind: "state", id: "Locked" });
  assert.equal(range?.startLine, 4);
  assert.equal(range?.startColumn, 3);
});

test("findSourceRangeForSelection locates quoted states", () => {
  const range = findSourceRangeForSelection(source, { kind: "state", id: "Quoted State" });
  assert.equal(range?.startLine, 7);
});

test("findSourceRangeForSelection locates transitions", () => {
  const range = findSourceRangeForSelection(source, { kind: "transition", id: "Withdraw" });
  assert.equal(range?.startLine, 13);
});

test("findSourceRangeForSelection locates invariants", () => {
  const range = findSourceRangeForSelection(source, { kind: "invariant", id: "withdrawal_requires_owner" });
  assert.equal(range?.startLine, 18);
});

test("findSourceRangeForSelection returns undefined for unsupported selections", () => {
  const range = findSourceRangeForSelection(source, { kind: "artefact", path: "contracts/aiken.toml" });
  assert.equal(range, undefined);
});

test("findSourceRangeForDiagnostic locates transition diagnostics", () => {
  const range = findSourceRangeForDiagnostic(source, {
    message: "Transition `Withdraw` references unknown input state `Missing`.",
  });
  assert.equal(range?.startLine, 13);
});

test("findSourceRangeForDiagnostic locates explicit target diagnostics", () => {
  const range = findSourceRangeForDiagnostic(source, {
    target: "state:Locked",
    message: "State shape needs review.",
  });
  assert.equal(range?.startLine, 4);
});

test("findSourceRangeForDiagnostic locates structured target diagnostics", () => {
  const range = findSourceRangeForDiagnostic(source, {
    target: {
      kind: "transition",
      name: "Withdraw",
      field: "constraints",
      line: null,
      column: null,
    },
    message: "Transition shape needs review.",
  });
  assert.equal(range?.startLine, 13);
});

test("attachSourceRangesToDiagnostics preserves existing ranges", () => {
  const diagnostics = attachSourceRangesToDiagnostics(source, [
    {
      severity: "error",
      message: "Transition `Withdraw` references unknown input state `Missing`.",
      range: { startLine: 1, startColumn: 1, endLine: 1, endColumn: 9 },
    },
  ]);
  assert.equal(diagnostics[0]?.range?.startLine, 1);
});
