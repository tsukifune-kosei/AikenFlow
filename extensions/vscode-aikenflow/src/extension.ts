import * as vscode from "vscode";
import { CompilerRunner } from "./cli/compilerRunner";
import { AikenFlowCommands } from "./commands";
import { DiagnosticPublisher } from "./diagnostics/diagnosticPublisher";
import { StatusBarController } from "./status/statusBarController";

export function activate(context: vscode.ExtensionContext): void {
  const output = vscode.window.createOutputChannel("AikenFlow");
  const diagnostics = new DiagnosticPublisher();
  const status = new StatusBarController();
  const runner = new CompilerRunner(context, output);
  const commands = new AikenFlowCommands(context, runner, diagnostics, status, output);

  context.subscriptions.push(output, diagnostics, status, ...commands.register());

  const saveSubscription = vscode.workspace.onDidSaveTextDocument((document) => {
    const enabled = vscode.workspace.getConfiguration("aikenflow").get("autoAnalyseOnSave", false);
    if (enabled && /(?:^|[/\\])(?:protocol|aikenflow)\.ya?ml$/.test(document.uri.fsPath)) {
      void vscode.commands.executeCommand("aikenflow.analyseProtocol", document.uri);
    }
  });
  context.subscriptions.push(saveSubscription);
}

export function deactivate(): void {}
