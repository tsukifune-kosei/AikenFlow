import * as assert from "node:assert/strict";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as vscode from "vscode";

let workspace: vscode.WorkspaceFolder;
let protocolUri: vscode.Uri;

type SmokeCase = {
  name: string;
  run(): Promise<void>;
};

const smokeCases: SmokeCase[] = [
  { name: "analyse protocol runs the real CLI and writes a bundle", run: analyseProtocolWritesBundle },
  { name: "generate artefacts runs the real CLI and writes protected outputs", run: generateArtefactsWritesProtectedOutputs },
  { name: "export audit report writes review-grade markdown", run: exportAuditWritesMarkdown },
  { name: "create agent context writes Codex context markdown", run: createAgentContextWritesMarkdown },
  { name: "missing compiler path fails without writing a bundle", run: missingCompilerPathFails },
  { name: "invalid bundle path is rejected without writing outside the workspace", run: invalidBundlePathIsRejected },
  { name: "invalid protocol publishes source diagnostics without writing a bundle", run: invalidProtocolPublishesDiagnostics },
];

export async function runSmokeTests(): Promise<void> {
  console.log("AikenFlow VS Code extension");
  for (const smokeCase of smokeCases) {
    await setupWorkspace();
    await smokeCase.run();
    console.log(`  ✔ ${smokeCase.name}`);
  }
}

async function setupWorkspace(): Promise<void> {
  const activeWorkspace = vscode.workspace.workspaceFolders?.[0];
  assert.ok(activeWorkspace, "fixture workspace should be open");
  workspace = activeWorkspace;
  protocolUri = vscode.Uri.joinPath(workspace.uri, "protocol.yaml");
  await fs.writeFile(protocolUri.fsPath, validProtocolYaml(), "utf8");

  const extensionRoot = path.resolve(__dirname, "..", "..", "..");
  const repoRoot = path.resolve(extensionRoot, "..", "..");
  const compilerPath = path.join(repoRoot, "target", "debug", process.platform === "win32" ? "aikenflow.exe" : "aikenflow");

  const config = vscode.workspace.getConfiguration("aikenflow");
  await config.update("backendPath", compilerPath, vscode.ConfigurationTarget.Workspace);
  await config.update("compilerPath", "", vscode.ConfigurationTarget.Workspace);
  await config.update("bundlePath", ".aikenflow/bundle.json", vscode.ConfigurationTarget.Workspace);
  await config.update("generatedOutputDir", "generated", vscode.ConfigurationTarget.Workspace);
  await config.update("auditOutputPath", ".aikenflow/AUDIT.md", vscode.ConfigurationTarget.Workspace);
  await config.update("agentContextPath", ".aikenflow/agent-context.md", vscode.ConfigurationTarget.Workspace);
}

async function analyseProtocolWritesBundle(): Promise<void> {
  const bundleUri = vscode.Uri.joinPath(workspace.uri, ".aikenflow", "bundle.json");
  await removeIfExists(vscode.Uri.joinPath(workspace.uri, ".aikenflow"));

  await vscode.commands.executeCommand("aikenflow.analyseProtocol", protocolUri);
  await waitForFile(bundleUri.fsPath, 20_000);

  const bundle = JSON.parse(await fs.readFile(bundleUri.fsPath, "utf8")) as {
    schemaVersion?: string;
    protocol?: { name?: string };
    graph?: { transactions?: unknown[]; edges?: unknown[]; transitions?: unknown[] };
    findings?: unknown[];
  };

  assert.equal(bundle.schemaVersion, "0.1.0");
  assert.equal(bundle.protocol?.name, "SimpleVault");
  assert.ok((bundle.graph?.transactions?.length ?? 0) > 0, "bundle should contain transaction nodes");
  assert.ok((bundle.graph?.edges?.length ?? 0) > 0, "bundle should contain topology edges");
  assert.ok((bundle.graph?.transitions?.length ?? 0) > 0, "bundle should contain transitions");
  assert.ok((bundle.findings?.length ?? 0) > 0, "bundle should contain Rust-generated findings");
}

async function generateArtefactsWritesProtectedOutputs(): Promise<void> {
  const generatedUri = vscode.Uri.joinPath(workspace.uri, "generated");
  const manifestUri = vscode.Uri.joinPath(generatedUri, ".aikenflow-generated.json");
  const adversarialUri = vscode.Uri.joinPath(generatedUri, "assurance", "tests", "adversarial.spec.ts");
  await removeIfExists(generatedUri);

  await vscode.commands.executeCommand("aikenflow.generateArtefacts", protocolUri);
  await waitForFile(manifestUri.fsPath, 20_000);
  await waitForFile(adversarialUri.fsPath, 20_000);

  const manifest = JSON.parse(await fs.readFile(manifestUri.fsPath, "utf8")) as {
    version?: number;
    files?: unknown[];
  };
  const adversarial = await fs.readFile(adversarialUri.fsPath, "utf8");

  assert.equal(manifest.version, 2);
  assert.ok((manifest.files?.length ?? 0) > 0, "manifest should list generated files");
  assert.match(adversarial, /assert\.fail/);
  assert.doesNotMatch(adversarial, /assert\.ok\(true/);
}

async function exportAuditWritesMarkdown(): Promise<void> {
  const auditUri = vscode.Uri.joinPath(workspace.uri, ".aikenflow", "AUDIT.md");
  await removeIfExists(auditUri);

  await vscode.commands.executeCommand("aikenflow.exportAuditReport", protocolUri);
  await waitForFile(auditUri.fsPath, 20_000);

  const audit = await fs.readFile(auditUri.fsPath, "utf8");
  assert.match(audit, /# Protocol Audit Artefact/);
  assert.match(audit, /not formal proofs/i);
}

async function createAgentContextWritesMarkdown(): Promise<void> {
  const contextUri = vscode.Uri.joinPath(workspace.uri, ".aikenflow", "agent-context.md");
  await removeIfExists(contextUri);

  await vscode.commands.executeCommand("aikenflow.createAgentContext", workspace.uri);
  await waitForFile(contextUri.fsPath, 20_000);

  const context = await fs.readFile(contextUri.fsPath, "utf8");
  assert.match(context, /# AikenFlow Agent Context/);
  assert.match(context, /ast-outline|Aiken Files|Warnings/);
}

async function missingCompilerPathFails(): Promise<void> {
  const bundleUri = vscode.Uri.joinPath(workspace.uri, ".aikenflow", "bundle.json");
  await removeIfExists(vscode.Uri.joinPath(workspace.uri, ".aikenflow"));
  await vscode.workspace
    .getConfiguration("aikenflow")
    .update(
      "compilerPath",
      "",
      vscode.ConfigurationTarget.Workspace,
    );
  await vscode.workspace
    .getConfiguration("aikenflow")
    .update(
      "backendPath",
      path.join(workspace.uri.fsPath, "missing-aikenflow-binary"),
      vscode.ConfigurationTarget.Workspace,
    );

  await vscode.commands.executeCommand("aikenflow.analyseProtocol", protocolUri);

  await assert.rejects(() => fs.stat(bundleUri.fsPath));
  assert.deepEqual(vscode.languages.getDiagnostics(protocolUri), []);
}

async function invalidBundlePathIsRejected(): Promise<void> {
  const outsideUri = vscode.Uri.joinPath(workspace.uri, "..", "aikenflow-outside-bundle.json");
  await removeIfExists(vscode.Uri.joinPath(workspace.uri, ".aikenflow"));
  await removeIfExists(outsideUri);
  await vscode.workspace
    .getConfiguration("aikenflow")
    .update("bundlePath", "../aikenflow-outside-bundle.json", vscode.ConfigurationTarget.Workspace);

  await vscode.commands.executeCommand("aikenflow.analyseProtocol", protocolUri);

  await assert.rejects(() => fs.stat(outsideUri.fsPath));
  await assert.rejects(() => fs.stat(vscode.Uri.joinPath(workspace.uri, ".aikenflow", "bundle.json").fsPath));
}

async function invalidProtocolPublishesDiagnostics(): Promise<void> {
  const bundleUri = vscode.Uri.joinPath(workspace.uri, ".aikenflow", "bundle.json");
  await removeIfExists(vscode.Uri.joinPath(workspace.uri, ".aikenflow"));
  await fs.writeFile(protocolUri.fsPath, invalidProtocolYaml(), "utf8");

  await vscode.commands.executeCommand("aikenflow.analyseProtocol", protocolUri);

  const diagnostics = vscode.languages.getDiagnostics(protocolUri);
  assert.ok(diagnostics.length > 0, "invalid protocol should publish diagnostics");
  assert.equal(diagnostics[0]?.source, "AikenFlow");
  assert.equal(diagnostics[0]?.range.start.line, 8);
  await assert.rejects(() => fs.stat(bundleUri.fsPath));
}

async function removeIfExists(uri: vscode.Uri): Promise<void> {
  await fs.rm(uri.fsPath, { recursive: true, force: true });
}

async function waitForFile(filePath: string, timeoutMs: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      await fs.stat(filePath);
      return;
    } catch {
      await delay(250);
    }
  }
  throw new Error(`Timed out waiting for ${filePath}`);
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function validProtocolYaml(): string {
  return `protocol: SimpleVault
description: Simple custody workflow with owner authorization and deadline-based withdrawal.

assets:
  - name: ada
    kind: lovelace

states:
  Locked:
    datum:
      owner: PubKeyHash
      amount: Lovelace
      deadline: POSIXTime

transitions:
  Deposit:
    inputs: []
    outputs:
      - state: Locked
        value:
          ada: "$amount"
    constraints:
      - signed_by: "$owner"
      - positive: "$amount"

  Withdraw:
    inputs:
      - state: Locked
    outputs: []
    constraints:
      - signed_by: "datum.owner"
      - after: "datum.deadline"

invariants:
  - name: withdrawal_requires_owner
    expression: "transition.Withdraw requires signature(datum.owner)"
`;
}

function invalidProtocolYaml(): string {
  return `protocol: BadVault

states:
  Locked:
    datum:
      owner: PubKeyHash

transitions:
  Withdraw:
    inputs:
      - state: Missing
    outputs: []
    constraints: []
`;
}
