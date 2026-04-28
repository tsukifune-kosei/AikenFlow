import * as vscode from "vscode";
import { nonce, webviewUri } from "./webviewResource";

export function renderWebviewHtml(context: vscode.ExtensionContext, webview: vscode.Webview): string {
  const scriptNonce = nonce();
  const scriptUri = webviewUri(context, webview, "media", "webview", "assets", "index.js");
  const styleUri = webviewUri(context, webview, "media", "webview", "assets", "index.css");
  const csp = [
    "default-src 'none'",
    `img-src ${webview.cspSource} data:`,
    `style-src ${webview.cspSource} 'unsafe-inline'`,
    `font-src ${webview.cspSource}`,
    `script-src 'nonce-${scriptNonce}'`,
  ].join("; ");

  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta http-equiv="Content-Security-Policy" content="${csp}" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="stylesheet" href="${styleUri}" />
    <title>AikenFlow Protocol Cockpit</title>
  </head>
  <body>
    <div id="root"></div>
    <script nonce="${scriptNonce}" type="module" src="${scriptUri}"></script>
  </body>
</html>`;
}
