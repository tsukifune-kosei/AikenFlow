import * as vscode from "vscode";

export function webviewUri(
  context: vscode.ExtensionContext,
  webview: vscode.Webview,
  ...segments: string[]
): vscode.Uri {
  return webview.asWebviewUri(vscode.Uri.joinPath(context.extensionUri, ...segments));
}

export function nonce(): string {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  let value = "";
  for (let index = 0; index < 32; index += 1) {
    value += alphabet.charAt(Math.floor(Math.random() * alphabet.length));
  }
  return value;
}
