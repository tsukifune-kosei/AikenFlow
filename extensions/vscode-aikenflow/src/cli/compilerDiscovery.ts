import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import { readConfiguration } from "../configuration";

export function resolveCompilerPath(context: vscode.ExtensionContext): string {
  const configuration = readConfiguration();
  const backendPath = configuration.backendPath.trim();
  if (backendPath) {
    return backendPath;
  }

  const environmentPath = process.env.AIKENFLOW_BIN?.trim();
  if (environmentPath) {
    return environmentPath;
  }

  const legacyCompilerPath = configuration.compilerPath.trim();
  if (legacyCompilerPath) {
    return legacyCompilerPath;
  }

  const bundled = bundledCompilerPath(context);
  if (bundled && fs.existsSync(bundled)) {
    return bundled;
  }

  return process.platform === "win32" ? "aikenflow.exe" : "aikenflow";
}

function bundledCompilerPath(context: vscode.ExtensionContext): string | undefined {
  const platform = `${process.platform}-${process.arch}`;
  const executable = process.platform === "win32" ? "aikenflow.exe" : "aikenflow";
  return path.join(context.extensionPath, "bin", platform, executable);
}
