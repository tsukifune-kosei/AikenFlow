import * as path from "node:path";
import * as vscode from "vscode";
import { safeRelativePathParts } from "./pathSafety";

export function workspaceFolderFor(uri: vscode.Uri): vscode.WorkspaceFolder {
  const folder = vscode.workspace.getWorkspaceFolder(uri);
  if (!folder) {
    throw new Error("AikenFlow requires the protocol file to be inside a VS Code workspace.");
  }
  return folder;
}

export function workspaceRelativeUri(folder: vscode.WorkspaceFolder, relativePath: string): vscode.Uri {
  return safeRelativeUri(folder.uri, relativePath, "workspace setting");
}

export function safeRelativeUri(baseUri: vscode.Uri, relativePath: string, label: string): vscode.Uri {
  const parts = safeRelativePathParts(relativePath, label);
  return vscode.Uri.joinPath(baseUri, ...parts);
}

export async function ensureParentDirectory(uri: vscode.Uri): Promise<void> {
  const parent = vscode.Uri.file(path.dirname(uri.fsPath));
  await vscode.workspace.fs.createDirectory(parent);
}

export async function exists(uri: vscode.Uri): Promise<boolean> {
  try {
    await vscode.workspace.fs.stat(uri);
    return true;
  } catch {
    return false;
  }
}
