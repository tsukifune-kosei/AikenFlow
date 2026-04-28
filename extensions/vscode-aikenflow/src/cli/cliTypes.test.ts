import test from "node:test";
import assert from "node:assert/strict";
import { parseDiagnosticsJson } from "./cliTypes";

test("parseDiagnosticsJson parses compiler diagnostics", () => {
  const diagnostics = parseDiagnosticsJson(JSON.stringify({
    ok: false,
    diagnostics: [
      {
        severity: "Warning",
        code: "AF050",
        message: "Review required.",
        hint: "Inspect transition topology.",
        target: {
          kind: "transition",
          name: "Withdraw",
          field: "constraints",
          line: null,
          column: null,
        },
      },
    ],
  }));
  assert.equal(diagnostics.length, 1);
  assert.equal(diagnostics[0]?.severity, "warning");
  assert.equal(diagnostics[0]?.code, "AF050");
  assert.deepEqual(diagnostics[0]?.target, {
    kind: "transition",
    name: "Withdraw",
    field: "constraints",
    line: null,
    column: null,
  });
});

test("parseDiagnosticsJson keeps legacy diagnostics array compatibility", () => {
  const diagnostics = parseDiagnosticsJson(JSON.stringify([
    {
      severity: "error",
      code: "AF024",
      message: "Transition `Withdraw` references unknown input state `Missing`.",
      hint: "Declare the state under `states` or fix the transition input.",
    },
  ]));
  assert.equal(diagnostics[0]?.severity, "error");
});

test("parseDiagnosticsJson rejects malformed output", () => {
  assert.throws(() => parseDiagnosticsJson("{}"), /diagnostics object/);
});
