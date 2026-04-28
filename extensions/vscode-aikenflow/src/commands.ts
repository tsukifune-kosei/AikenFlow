import * as vscode from "vscode";
import { readFile } from "node:fs/promises";
import { CompilerRunner } from "./cli/compilerRunner";
import { parseDiagnosticsJson } from "./cli/cliTypes";
import { readConfiguration } from "./configuration";
import { DiagnosticPublisher } from "./diagnostics/diagnosticPublisher";
import { StatusBarController } from "./status/statusBarController";
import type { AikenFlowBundle, AikenFlowDiagnostic, ProtocolSelection } from "./shared/aikenflowTypes";
import { isAikenFlowBundle } from "./shared/aikenflowTypes";
import { attachSourceRangesToDiagnostics, findSourceRangeForSelection } from "./shared/sourceLocator";
import { ProtocolCockpitPanel, type CockpitHandlers } from "./webview/ProtocolCockpitPanel";
import { locateProtocolUri } from "./workspace/protocolLocator";
import { ensureParentDirectory, exists, safeRelativeUri, workspaceFolderFor, workspaceRelativeUri } from "./workspace/workspacePaths";

export class AikenFlowCommands {
  private currentProtocolUri: vscode.Uri | undefined;
  private currentBundle: AikenFlowBundle | undefined;
  private currentBundleSourceFsPath: string | undefined;

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly runner: CompilerRunner,
    private readonly diagnostics: DiagnosticPublisher,
    private readonly status: StatusBarController,
    private readonly output: vscode.OutputChannel,
  ) {}

  register(): vscode.Disposable[] {
    return [
      vscode.commands.registerCommand("aikenflow.analyseProtocol", (uri?: vscode.Uri) => this.analyseProtocol(uri)),
      vscode.commands.registerCommand("aikenflow.openProtocolCockpit", (uri?: vscode.Uri) => this.openProtocolCockpit(uri)),
      vscode.commands.registerCommand("aikenflow.generateArtefacts", (uri?: vscode.Uri) => this.generateArtefacts(uri)),
      vscode.commands.registerCommand("aikenflow.exportAuditReport", (uri?: vscode.Uri) => this.exportAuditReport(uri)),
      vscode.commands.registerCommand("aikenflow.createAgentContext", (uri?: vscode.Uri) => this.createAgentContext(uri)),
      vscode.commands.registerCommand("aikenflow.selectBackendBinary", () => this.selectBackendBinary()),
      vscode.commands.registerCommand("aikenflow.configureCompilerPath", () => this.selectBackendBinary()),
    ];
  }

  async analyseProtocol(candidate?: vscode.Uri): Promise<void> {
    const protocolUri = await locateProtocolUri(candidate);
    if (!protocolUri) return;
    const folder = workspaceFolderFor(protocolUri);
    const config = readConfiguration();
    const bundleUri = this.resolveWorkspacePath(folder, config.bundlePath, "bundlePath");
    if (!bundleUri) return;
    await ensureParentDirectory(bundleUri);

    this.currentProtocolUri = protocolUri;
    this.status.showAnalysing();
    const panel = this.openPanel();
    panel.post({ type: "analysis.started", protocolUri: protocolUri.toString() });

    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: "AikenFlow: analysing protocol",
        cancellable: true,
      },
      async (_progress, token) => {
        try {
          const checkResult = await this.runner.run(["check", protocolUri.fsPath, "--json"], folder.uri.fsPath, token);
          const protocolSource = await readFile(protocolUri.fsPath, "utf8");
          const checkDiagnostics = attachSourceRangesToDiagnostics(
            protocolSource,
            parseCheckDiagnostics(checkResult.stdout, this.output),
          );
          this.diagnostics.publish(protocolUri, checkDiagnostics);

          if (checkResult.timedOut) {
            throw new Error("AikenFlow analysis timed out.");
          }

          const hasErrors = checkDiagnostics.some((diagnostic) => diagnostic.severity === "error");
          if (checkResult.exitCode !== 0 || hasErrors) {
            const message = checkDiagnostics.length > 0
              ? "AikenFlow check failed. Open the Protocol Cockpit or Output panel for details."
              : cliFailureMessage(checkResult.stderr, "AikenFlow check failed.");
            this.status.showError(message);
            panel.post({ type: "analysis.failed", message, stderr: checkResult.stderr });
            void vscode.window.showErrorMessage(message);
            return;
          }

          const exportResult = await this.runner.run(
            ["export", protocolUri.fsPath, "--out", bundleUri.fsPath],
            folder.uri.fsPath,
            token,
          );
          if (exportResult.exitCode !== 0 || exportResult.timedOut) {
            throw new Error(cliFailureMessage(exportResult.stderr, "AikenFlow export failed."));
          }

          const bundle = await readBundle(bundleUri);
          this.currentBundle = bundle;
          this.currentBundleSourceFsPath = protocolUri.fsPath;
          this.publishBundleDiagnostics(protocolUri, protocolSource, bundle);
          this.status.showBundle(bundle);
          panel.updateBundle(bundle, protocolUri, exportResult.elapsedMs);
        } catch (error) {
          const message = error instanceof Error ? error.message : "AikenFlow analysis failed.";
          this.status.showError(message);
          panel.post({ type: "analysis.failed", message });
          void vscode.window.showErrorMessage(message);
        }
      },
    );
  }

  async openProtocolCockpit(candidate?: vscode.Uri): Promise<void> {
    const protocolUri = await locateProtocolUri(candidate ?? this.currentProtocolUri);
    if (!protocolUri) return;
    this.currentProtocolUri = protocolUri;
    const panel = this.openPanel();

    if (this.currentBundle && this.currentBundleSourceFsPath === protocolUri.fsPath) {
      await this.publishCurrentBundleDiagnostics(protocolUri, this.currentBundle);
      panel.loadBundle(this.currentBundle, protocolUri);
      return;
    }

    const folder = workspaceFolderFor(protocolUri);
    const bundleUri = this.resolveWorkspacePath(folder, readConfiguration().bundlePath, "bundlePath");
    if (!bundleUri) return;
    if (await exists(bundleUri)) {
      const bundle = await readBundle(bundleUri);
      this.currentBundle = bundle;
      this.currentBundleSourceFsPath = protocolUri.fsPath;
      this.status.showBundle(bundle);
      await this.publishCurrentBundleDiagnostics(protocolUri, bundle);
      panel.loadBundle(bundle, protocolUri);
      return;
    }

    await this.analyseProtocol(protocolUri);
  }

  async generateArtefacts(candidate?: vscode.Uri): Promise<void> {
    const protocolUri = await locateProtocolUri(candidate ?? this.currentProtocolUri);
    if (!protocolUri) return;
    const folder = workspaceFolderFor(protocolUri);
    const outDir = this.resolveWorkspacePath(folder, readConfiguration().generatedOutputDir, "generatedOutputDir");
    if (!outDir) return;
    await vscode.workspace.fs.createDirectory(outDir);

    const result = await this.runner.run(["gen", "all", protocolUri.fsPath, "--out", outDir.fsPath], folder.uri.fsPath);
    if (result.exitCode !== 0 || result.timedOut) {
      const message = cliFailureMessage(result.stderr, "AikenFlow artefact generation failed.");
      this.status.showError(message);
      void vscode.window.showErrorMessage(message);
      return;
    }
    void vscode.window.showInformationMessage(`AikenFlow generated artefacts into ${vscode.workspace.asRelativePath(outDir)}.`);
  }

  async exportAuditReport(candidate?: vscode.Uri): Promise<void> {
    const protocolUri = await locateProtocolUri(candidate ?? this.currentProtocolUri);
    if (!protocolUri) return;
    const folder = workspaceFolderFor(protocolUri);
    const auditUri = this.resolveWorkspacePath(folder, readConfiguration().auditOutputPath, "auditOutputPath");
    if (!auditUri) return;
    await ensureParentDirectory(auditUri);

    const result = await this.runner.run(["audit", protocolUri.fsPath, "--out", auditUri.fsPath], folder.uri.fsPath);
    if (result.exitCode !== 0 || result.timedOut) {
      const message = cliFailureMessage(result.stderr, "AikenFlow audit export failed.");
      this.status.showError(message);
      void vscode.window.showErrorMessage(message);
      return;
    }
    const document = await vscode.workspace.openTextDocument(auditUri);
    await vscode.window.showTextDocument(document, vscode.ViewColumn.Beside);
  }

  async createAgentContext(candidate?: vscode.Uri): Promise<void> {
    const workspaceUri = candidate && (await isDirectory(candidate)) ? candidate : vscode.workspace.workspaceFolders?.[0]?.uri;
    if (!workspaceUri) {
      void vscode.window.showWarningMessage("Open a workspace folder before creating AikenFlow agent context.");
      return;
    }
    const contextUri = this.resolveBasePath(workspaceUri, readConfiguration().agentContextPath, "agentContextPath");
    if (!contextUri) return;

    const result = await this.runner.run(["agent-context", workspaceUri.fsPath, "--for", "codex"], workspaceUri.fsPath);
    if (result.exitCode !== 0 || result.timedOut) {
      const message = cliFailureMessage(result.stderr, "AikenFlow agent-context failed. Check that ast-outline is installed or configured.");
      void vscode.window.showErrorMessage(message);
      return;
    }
    if (await exists(contextUri)) {
      const document = await vscode.workspace.openTextDocument(contextUri);
      await vscode.window.showTextDocument(document, vscode.ViewColumn.Beside);
    } else {
      void vscode.window.showWarningMessage(
        `AikenFlow completed but did not write ${vscode.workspace.asRelativePath(contextUri)}.`,
      );
    }
  }

  async selectBackendBinary(): Promise<void> {
    const selected = await vscode.window.showOpenDialog({
      title: "Select AikenFlow backend binary",
      canSelectFiles: true,
      canSelectFolders: false,
      canSelectMany: false,
      openLabel: "Use as aikenflow.backendPath",
    });
    const compilerUri = selected?.[0];
    if (!compilerUri) return;

    await vscode.workspace
      .getConfiguration("aikenflow")
      .update("backendPath", compilerUri.fsPath, vscode.ConfigurationTarget.Workspace);
    void vscode.window.showInformationMessage(`AikenFlow backend path set to ${compilerUri.fsPath}`);
  }

  private openPanel(): ProtocolCockpitPanel {
    const handlers: CockpitHandlers = {
      analyse: () => this.analyseProtocol(this.currentProtocolUri),
      generateArtefacts: () => this.generateArtefacts(this.currentProtocolUri),
      exportAudit: () => this.exportAuditReport(this.currentProtocolUri),
      revealSource: (selection) => this.revealSource(selection),
      revealArtefact: (path, line) => this.revealArtefact(path, line),
    };
    return ProtocolCockpitPanel.createOrShow(this.context, handlers);
  }

  private resolveWorkspacePath(folder: vscode.WorkspaceFolder, relativePath: string, label: string): vscode.Uri | undefined {
    return this.resolveBasePath(folder.uri, relativePath, label);
  }

  private resolveBasePath(baseUri: vscode.Uri, relativePath: string, label: string): vscode.Uri | undefined {
    try {
      return safeRelativeUri(baseUri, relativePath, label);
    } catch (error) {
      const message = error instanceof Error ? error.message : `Invalid AikenFlow ${label}.`;
      this.status.showError(message);
      void vscode.window.showErrorMessage(message);
      return undefined;
    }
  }

  private async publishCurrentBundleDiagnostics(protocolUri: vscode.Uri, bundle: AikenFlowBundle): Promise<void> {
    const protocolSource = await readFile(protocolUri.fsPath, "utf8");
    this.publishBundleDiagnostics(protocolUri, protocolSource, bundle);
  }

  private publishBundleDiagnostics(protocolUri: vscode.Uri, protocolSource: string, bundle: AikenFlowBundle): void {
    this.diagnostics.publish(
      protocolUri,
      attachSourceRangesToDiagnostics(protocolSource, bundle.diagnostics),
    );
  }

  private async revealSource(selection: ProtocolSelection): Promise<void> {
    if (!this.currentProtocolUri) return;
    const document = await vscode.workspace.openTextDocument(this.currentProtocolUri);
    const editor = await vscode.window.showTextDocument(document, vscode.ViewColumn.One);
    const range = findSourceRangeForSelection(document.getText(), selection);
    if (!range) return;
    const selectionRange = new vscode.Range(
      Math.max(0, range.startLine - 1),
      Math.max(0, range.startColumn - 1),
      Math.max(0, range.endLine - 1),
      Math.max(0, range.endColumn - 1),
    );
    editor.selection = new vscode.Selection(selectionRange.start, selectionRange.end);
    editor.revealRange(selectionRange, vscode.TextEditorRevealType.InCenter);
  }

  private async revealArtefact(relativePath: string, line?: number): Promise<void> {
    if (!this.currentProtocolUri) return;
    if (!this.currentBundle || !bundleContainsArtefact(this.currentBundle, relativePath)) {
      void vscode.window.showWarningMessage(`AikenFlow did not generate artefact in the current bundle: ${relativePath}`);
      return;
    }
    const folder = workspaceFolderFor(this.currentProtocolUri);
    const generatedRoot = this.resolveWorkspacePath(folder, readConfiguration().generatedOutputDir, "generatedOutputDir");
    if (!generatedRoot) return;
    const artefactUri = this.resolveBasePath(generatedRoot, relativePath, "artefact path");
    if (!artefactUri) return;
    if (!(await exists(artefactUri))) {
      void vscode.window.showWarningMessage(`Generated artefact does not exist on disk yet: ${relativePath}`);
      return;
    }
    const document = await vscode.workspace.openTextDocument(artefactUri);
    const editor = await vscode.window.showTextDocument(document, vscode.ViewColumn.Beside);
    if (line && line > 0) {
      const position = new vscode.Position(line - 1, 0);
      editor.selection = new vscode.Selection(position, position);
      editor.revealRange(new vscode.Range(position, position), vscode.TextEditorRevealType.InCenter);
    }
  }
}

function bundleContainsArtefact(bundle: AikenFlowBundle, relativePath: string): boolean {
  return Object.values(bundle.artefacts)
    .flat()
    .some((artefact) => artefact.path === relativePath);
}

async function readBundle(uri: vscode.Uri): Promise<AikenFlowBundle> {
  const content = await readFile(uri.fsPath, "utf8");
  const parsed = JSON.parse(content) as unknown;
  if (!isAikenFlowBundle(parsed)) {
    throw new Error(`Invalid AikenFlow bundle at ${uri.fsPath}.`);
  }
  return parsed;
}

function parseCheckDiagnostics(stdout: string, output: vscode.OutputChannel): AikenFlowDiagnostic[] {
  try {
    return parseDiagnosticsJson(stdout);
  } catch (error) {
    if (stdout.trim()) {
      output.appendLine("Could not parse AikenFlow check diagnostics JSON:");
      output.appendLine(stdout.trimEnd());
    }
    throw error;
  }
}

function cliFailureMessage(stderr: string, fallback: string): string {
  const message = stderr.trim();
  if (!message) return fallback;
  if (message.includes("ENOENT") || message.includes("not found") || message.includes("no such file")) {
    return `${message}\nInstall the AikenFlow CLI or set aikenflow.backendPath.`;
  }
  return message;
}

async function isDirectory(uri: vscode.Uri): Promise<boolean> {
  try {
    const stat = await vscode.workspace.fs.stat(uri);
    return stat.type === vscode.FileType.Directory;
  } catch {
    return false;
  }
}
