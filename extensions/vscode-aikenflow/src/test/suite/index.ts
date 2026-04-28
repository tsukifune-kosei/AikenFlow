import { runSmokeTests } from "./smoke.test";

export function run(): Promise<void> {
  return runSmokeTests();
}
