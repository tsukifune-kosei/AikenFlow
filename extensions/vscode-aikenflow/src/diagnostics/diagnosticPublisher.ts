import * as vscode from "vscode";
import type { AikenFlowDiagnostic } from "../shared/aikenflowTypes";
import { toVsCodeDiagnostic } from "./diagnosticMapper";

export class DiagnosticPublisher implements vscode.Disposable {
  private readonly collection = vscode.languages.createDiagnosticCollection("aikenflow");

  publish(protocolUri: vscode.Uri, diagnostics: AikenFlowDiagnostic[]): void {
    const mapped = diagnostics
      .map(toVsCodeDiagnostic)
      .filter((diagnostic): diagnostic is vscode.Diagnostic => Boolean(diagnostic));
    this.collection.set(protocolUri, mapped);
  }

  clear(protocolUri?: vscode.Uri): void {
    if (protocolUri) {
      this.collection.delete(protocolUri);
    } else {
      this.collection.clear();
    }
  }

  dispose(): void {
    this.collection.dispose();
  }
}
