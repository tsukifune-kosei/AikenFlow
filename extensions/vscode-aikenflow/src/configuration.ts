import * as vscode from "vscode";

export type AikenFlowConfiguration = {
  backendPath: string;
  compilerPath: string;
  bundlePath: string;
  generatedOutputDir: string;
  auditOutputPath: string;
  agentContextPath: string;
  autoAnalyseOnSave: boolean;
  analysisTimeoutMs: number;
};

export function readConfiguration(): AikenFlowConfiguration {
  const config = vscode.workspace.getConfiguration("aikenflow");
  return {
    backendPath: config.get("backendPath", ""),
    compilerPath: config.get("compilerPath", ""),
    bundlePath: config.get("bundlePath", ".aikenflow/bundle.json"),
    generatedOutputDir: config.get("generatedOutputDir", "generated"),
    auditOutputPath: config.get("auditOutputPath", ".aikenflow/AUDIT.md"),
    agentContextPath: config.get("agentContextPath", ".aikenflow/agent-context.md"),
    autoAnalyseOnSave: config.get("autoAnalyseOnSave", false),
    analysisTimeoutMs: config.get("analysisTimeoutMs", 30_000),
  };
}
