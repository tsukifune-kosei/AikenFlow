import type { WebviewToExtension } from "../../src/shared/webviewMessages";

type VsCodeApi = {
  postMessage(message: WebviewToExtension): void;
  getState(): unknown;
  setState(state: unknown): void;
};

declare const acquireVsCodeApi: (() => VsCodeApi) | undefined;

let api: VsCodeApi | undefined;

export function vscodeApi(): VsCodeApi | undefined {
  if (api) return api;
  if (typeof acquireVsCodeApi === "function") {
    api = acquireVsCodeApi();
  }
  return api;
}

export function postToExtension(message: WebviewToExtension): void {
  vscodeApi()?.postMessage(message);
}
