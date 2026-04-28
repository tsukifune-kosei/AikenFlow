import * as vscode from "vscode";
import type { AikenFlowBundle, AikenFlowDiagnostic } from "../shared/aikenflowTypes";

export class StatusBarController implements vscode.Disposable {
  private readonly item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 90);

  constructor() {
    this.item.command = "aikenflow.openProtocolCockpit";
    this.showIdle();
  }

  showIdle(): void {
    this.item.text = "$(graph) AikenFlow: idle";
    this.item.tooltip = "Open AikenFlow Protocol Cockpit";
    this.item.show();
  }

  showAnalysing(): void {
    this.item.text = "$(sync~spin) AikenFlow: analysing...";
    this.item.tooltip = "AikenFlow is running the Rust compiler.";
    this.item.show();
  }

  showError(message: string): void {
    this.item.text = "$(error) AikenFlow: error";
    this.item.tooltip = message;
    this.item.show();
  }

  showBundle(bundle: AikenFlowBundle, diagnostics: AikenFlowDiagnostic[] = bundle.diagnostics): void {
    const warnings = diagnostics.filter((diagnostic) => diagnostic.severity === "warning").length;
    const errors = diagnostics.filter((diagnostic) => diagnostic.severity === "error").length;
    const risk = bundle.metrics.riskLevel;
    this.item.text = errors > 0
      ? `$(error) AikenFlow: ${errors} errors`
      : `$(shield) AikenFlow: ${risk} risk · ${warnings} warnings`;
    this.item.tooltip = `${bundle.protocol.name}: ${bundle.metrics.invariantCoverage}% invariant coverage, ${bundle.metrics.generatedFiles} generated files`;
    this.item.show();
  }

  dispose(): void {
    this.item.dispose();
  }
}
