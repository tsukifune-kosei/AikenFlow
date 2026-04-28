import * as path from "node:path";
import * as vscode from "vscode";

export async function locateProtocolUri(candidate?: vscode.Uri): Promise<vscode.Uri | undefined> {
  if (candidate && isProtocolFile(candidate)) {
    return candidate;
  }

  const active = vscode.window.activeTextEditor?.document.uri;
  if (active && isProtocolFile(active)) {
    return active;
  }

  const configured = vscode.workspace.getConfiguration("aikenflow").get("protocolSpecPath", "").trim();
  if (configured && vscode.workspace.workspaceFolders?.[0]) {
    const configuredUri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders[0].uri, ...configured.split(/[\\/]+/));
    if (await exists(configuredUri)) return configuredUri;
  }

  const matches = await vscode.workspace.findFiles(
    "**/{protocol.yaml,protocol.yml,aikenflow.yaml,aikenflow.yml}",
    "**/{node_modules,target,dist,out}/**",
    12,
  );
  if (matches.length === 0) {
    return offerStarterProtocol();
  }
  if (matches.length === 1) {
    return matches[0];
  }

  const picked = await vscode.window.showQuickPick(
    matches.map((uri) => ({
      label: vscode.workspace.asRelativePath(uri),
      uri,
    })),
    { title: "Select AikenFlow protocol spec" },
  );
  return picked?.uri;
}

function isProtocolFile(uri: vscode.Uri): boolean {
  const filename = path.basename(uri.fsPath);
  if (filename === "protocol.yaml" || filename === "protocol.yml" || filename === "aikenflow.yaml" || filename === "aikenflow.yml") {
    return true;
  }
  const configured = vscode.workspace.getConfiguration("aikenflow").get("protocolSpecPath", "").trim();
  return Boolean(configured && vscode.workspace.asRelativePath(uri) === configured);
}

async function offerStarterProtocol(): Promise<vscode.Uri | undefined> {
  const workspace = vscode.workspace.workspaceFolders?.[0];
  if (!workspace) {
    void vscode.window.showWarningMessage("Open a workspace folder before running AikenFlow.");
    return undefined;
  }

  const create = "Create protocol.yaml";
  const selected = await vscode.window.showWarningMessage(
    "No AikenFlow protocol spec found in the current workspace.",
    create,
  );
  if (selected !== create) return undefined;

  const uri = vscode.Uri.joinPath(workspace.uri, "protocol.yaml");
  if (!(await exists(uri))) {
    await vscode.workspace.fs.writeFile(uri, Buffer.from(starterProtocolYaml(), "utf8"));
  }
  const document = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(document);
  return uri;
}

async function exists(uri: vscode.Uri): Promise<boolean> {
  try {
    await vscode.workspace.fs.stat(uri);
    return true;
  } catch {
    return false;
  }
}

function starterProtocolYaml(): string {
  return `protocol: StarterProtocol
description: A minimal AikenFlow protocol spec.

assets:
  - name: ada
    kind: lovelace

states:
  Locked:
    datum:
      owner: PubKeyHash
      amount: Lovelace

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

invariants:
  - name: owner_review_required
    expression: "manual review required"
`;
}
