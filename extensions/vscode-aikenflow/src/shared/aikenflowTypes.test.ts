import assert from "node:assert/strict";
import test from "node:test";
import { isAikenFlowBundle, type AikenFlowBundle } from "./aikenflowTypes";

test("isAikenFlowBundle accepts valid exported bundle shape", () => {
  assert.equal(isAikenFlowBundle(validBundle()), true);
});

test("isAikenFlowBundle rejects malformed artefacts", () => {
  const bundle = validBundle();
  bundle.artefacts.aiken = [
    {
      path: "contracts/aiken.toml",
      language: "unsupported",
      content: "name = \"bad\"",
    } as never,
  ];

  assert.equal(isAikenFlowBundle(bundle), false);
});

test("isAikenFlowBundle rejects unsafe artefact paths", () => {
  const bundle = validBundle();
  bundle.artefacts.audit = [
    {
      path: "../AUDIT.md",
      language: "markdown",
      content: "# bad\n",
    },
  ];

  assert.equal(isAikenFlowBundle(bundle), false);
});

test("isAikenFlowBundle rejects malformed graph entries", () => {
  const bundle = validBundle();
  bundle.graph.transitions = [
    {
      id: "Withdraw",
      label: "Withdraw",
      from: ["Locked"],
      to: [],
      constraints: [],
      riskLevel: "critical",
    } as never,
  ];

  assert.equal(isAikenFlowBundle(bundle), false);
});

function validBundle(): AikenFlowBundle {
  return {
    schemaVersion: "0.1.0",
    protocol: {
      name: "SimpleVault",
      sourceYaml: "protocol: SimpleVault\n",
    },
    graph: {
      states: [
        {
          id: "Locked",
          label: "Locked",
          datumFields: [{ name: "owner", type: "PubKeyHash" }],
        },
      ],
      transactions: [
        {
          id: "tx:Withdraw",
          label: "Withdraw",
          transitionId: "Withdraw",
          constraints: [{ kind: "signed_by", label: "signed_by(datum.owner)" }],
          riskLevel: "low",
        },
      ],
      edges: [
        {
          id: "state:Locked->Withdraw",
          source: "state:Locked",
          target: "tx:Withdraw",
          kind: "consumes",
          label: "Withdraw",
          riskLevel: "low",
        },
      ],
      transitions: [
        {
          id: "Withdraw",
          label: "Withdraw",
          from: ["Locked"],
          to: [],
          constraints: [{ kind: "signed_by", label: "signed_by(datum.owner)" }],
          riskLevel: "low",
        },
      ],
    },
    diagnostics: [
      {
        severity: "warning",
        message: "Review required.",
        range: { startLine: 1, startColumn: 1, endLine: 1, endColumn: 9 },
      },
    ],
    invariants: [
      {
        name: "withdrawal_requires_owner",
        expression: "transition.Withdraw requires signature(datum.owner)",
        status: "partial",
        transitions: ["Withdraw"],
      },
    ],
    metrics: {
      stateCoverage: 100,
      transitionCoverage: 100,
      invariantCoverage: 50,
      generatedFiles: 4,
      riskLevel: "medium",
    },
    findings: [
      {
        id: "invariant:withdrawal_requires_owner:0",
        severity: "medium",
        title: "withdrawal_requires_owner is partial",
        detail: "Mapped to Withdraw.",
        transitionId: "Withdraw",
      },
    ],
    artefacts: {
      aiken: [{ path: "contracts/aiken.toml", language: "toml", content: "" }],
      lucid: [{ path: "offchain/src/index.ts", language: "typescript", content: "" }],
      tests: [{ path: "assurance/tests/adversarial.spec.ts", language: "typescript", content: "" }],
      audit: [{ path: "assurance/AUDIT.md", language: "markdown", content: "" }],
    },
  };
}
