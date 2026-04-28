import * as vscode from "vscode";
import type { AikenFlowBundle, ProtocolSelection } from "../shared/aikenflowTypes";
import type { ExtensionToWebview } from "../shared/webviewMessages";
import { isWebviewToExtensionMessage } from "../shared/webviewMessages";
import { renderWebviewHtml } from "./webviewHtml";

export type CockpitHandlers = {
  analyse(): Promise<void>;
  generateArtefacts(): Promise<void>;
  exportAudit(): Promise<void>;
  revealSource(selection: ProtocolSelection): Promise<void>;
  revealArtefact(path: string, line?: number): Promise<void>;
};

export class ProtocolCockpitPanel {
  static current: ProtocolCockpitPanel | undefined;

  private readonly disposables: vscode.Disposable[] = [];
  private bundle: AikenFlowBundle | undefined;
  private sourceUri: vscode.Uri | undefined;

  private constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly panel: vscode.WebviewPanel,
    private readonly handlers: CockpitHandlers,
  ) {
    panel.webview.html = renderWebviewHtml(context, panel.webview);
    panel.onDidDispose(() => this.dispose(), undefined, this.disposables);
    panel.webview.onDidReceiveMessage(
      (message: unknown) => this.handleMessage(message),
      undefined,
      this.disposables,
    );
  }

  static createOrShow(context: vscode.ExtensionContext, handlers: CockpitHandlers): ProtocolCockpitPanel {
    if (ProtocolCockpitPanel.current) {
      ProtocolCockpitPanel.current.panel.reveal(vscode.ViewColumn.Beside);
      return ProtocolCockpitPanel.current;
    }

    const panel = vscode.window.createWebviewPanel(
      "aikenflowProtocolCockpit",
      "AikenFlow Protocol Cockpit",
      vscode.ViewColumn.Beside,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [
          vscode.Uri.joinPath(context.extensionUri, "media", "webview"),
        ],
      },
    );

    ProtocolCockpitPanel.current = new ProtocolCockpitPanel(context, panel, handlers);
    return ProtocolCockpitPanel.current;
  }

  static existing(): ProtocolCockpitPanel | undefined {
    return ProtocolCockpitPanel.current;
  }

  updateBundle(bundle: AikenFlowBundle, sourceUri: vscode.Uri, elapsedMs = 0): void {
    this.bundle = bundle;
    this.sourceUri = sourceUri;
    this.post({
      type: "analysis.completed",
      bundle,
      sourceUri: sourceUri.toString(),
      elapsedMs,
    });
  }

  loadBundle(bundle: AikenFlowBundle, sourceUri: vscode.Uri): void {
    this.bundle = bundle;
    this.sourceUri = sourceUri;
    this.post({
      type: "bundle.loaded",
      bundle,
      sourceUri: sourceUri.toString(),
    });
  }

  post(message: ExtensionToWebview): void {
    void this.panel.webview.postMessage(message);
  }

  private handleMessage(message: unknown): void {
    if (!isWebviewToExtensionMessage(message)) return;

    switch (message.type) {
      case "command.analyse":
        void this.handlers.analyse();
        break;
      case "command.generateArtefacts":
        void this.handlers.generateArtefacts();
        break;
      case "command.exportAudit":
        void this.handlers.exportAudit();
        break;
      case "reveal.source":
        void this.handlers.revealSource(message.target);
        break;
      case "reveal.artefact":
        void this.handlers.revealArtefact(message.path, message.line);
        break;
      case "selection.changed":
        this.post({ type: "selection.changed", selection: message.selection });
        break;
    }
  }

  private dispose(): void {
    ProtocolCockpitPanel.current = undefined;
    while (this.disposables.length > 0) {
      this.disposables.pop()?.dispose();
    }
  }
}
