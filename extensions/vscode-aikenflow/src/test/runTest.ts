import * as path from "node:path";
import * as fs from "node:fs/promises";
import * as os from "node:os";
import { runTests } from "@vscode/test-electron";

async function main(): Promise<void> {
  const extensionDevelopmentPath = path.resolve(__dirname, "..", "..");
  const extensionTestsPath = path.resolve(__dirname, "suite", "index");
  const fixtureWorkspace = path.resolve(extensionDevelopmentPath, "fixtures", "workspaces", "simple-vault");
  const testWorkspace = await fs.mkdtemp(path.join(os.tmpdir(), "aikenflow-vscode-"));
  const userDataDir = await fs.mkdtemp(path.join(os.tmpdir(), "af-user-"));
  const extensionsDir = await fs.mkdtemp(path.join(os.tmpdir(), "af-ext-"));
  await fs.cp(fixtureWorkspace, testWorkspace, { recursive: true });

  try {
    await runTests({
      extensionDevelopmentPath,
      extensionTestsPath,
      launchArgs: [
        testWorkspace,
        "--disable-extensions",
        `--user-data-dir=${userDataDir}`,
        `--extensions-dir=${extensionsDir}`,
      ],
    });
  } finally {
    await Promise.all([
      fs.rm(testWorkspace, { recursive: true, force: true }),
      fs.rm(userDataDir, { recursive: true, force: true }),
      fs.rm(extensionsDir, { recursive: true, force: true }),
    ]);
  }
}

main().catch((error: unknown) => {
  console.error(error);
  process.exit(1);
});
