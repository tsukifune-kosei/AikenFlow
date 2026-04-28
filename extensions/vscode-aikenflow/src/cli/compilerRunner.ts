import { spawn } from "node:child_process";
import * as vscode from "vscode";
import type { CliRunResult } from "./cliTypes";
import { resolveCompilerPath } from "./compilerDiscovery";
import { readConfiguration } from "../configuration";

export class CompilerRunner {
  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly output: vscode.OutputChannel,
  ) {}

  run(args: string[], cwd: string, token?: vscode.CancellationToken): Promise<CliRunResult> {
    const command = resolveCompilerPath(this.context);
    const timeoutMs = readConfiguration().analysisTimeoutMs;
    const startedAt = Date.now();

    this.output.appendLine(`$ ${command} ${args.join(" ")}`);
    this.output.appendLine(`cwd: ${cwd}`);

    return new Promise((resolve) => {
      const child = spawn(command, args, {
        cwd,
        shell: false,
        windowsHide: true,
      });

      let stdout = "";
      let stderr = "";
      let settled = false;
      let timedOut = false;

      const timeout = setTimeout(() => {
        timedOut = true;
        child.kill("SIGTERM");
      }, timeoutMs);

      const cancellation = token?.onCancellationRequested(() => {
        child.kill("SIGTERM");
      });

      child.stdout.setEncoding("utf8");
      child.stderr.setEncoding("utf8");

      child.stdout.on("data", (chunk: string) => {
        stdout += chunk;
      });
      child.stderr.on("data", (chunk: string) => {
        stderr += chunk;
      });

      child.on("error", (error) => {
        if (settled) return;
        settled = true;
        clearTimeout(timeout);
        cancellation?.dispose();
        stderr += error.message;
        const result = {
          command,
          args,
          cwd,
          exitCode: null,
          stdout,
          stderr,
          elapsedMs: Date.now() - startedAt,
          timedOut,
        };
        appendResult(this.output, result);
        resolve(result);
      });

      child.on("close", (exitCode) => {
        if (settled) return;
        settled = true;
        clearTimeout(timeout);
        cancellation?.dispose();
        const result: CliRunResult = {
          command,
          args,
          cwd,
          exitCode,
          stdout,
          stderr,
          elapsedMs: Date.now() - startedAt,
          timedOut,
        };
        appendResult(this.output, result);
        resolve(result);
      });
    });
  }
}

function appendResult(output: vscode.OutputChannel, result: CliRunResult) {
  if (result.stdout.trim()) {
    output.appendLine(result.stdout.trimEnd());
  }
  if (result.stderr.trim()) {
    output.appendLine(result.stderr.trimEnd());
  }
  output.appendLine(`exit: ${result.exitCode ?? "spawn-error"} (${result.elapsedMs}ms)`);
}
