import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Phase =
  | "checkingNode"
  | "findingPort"
  | "starting"
  | "ready"
  | "failed"
  | "envMissing";

export interface RuntimeSnapshot {
  phase: Phase;
  port: number | null;
  detail: string;
  elapsed: number | null;
  lastLog: string;
}

export const state = ref<RuntimeSnapshot>({
  phase: "checkingNode",
  port: null,
  detail: "正在检测 Node.js 环境…",
  elapsed: null,
  lastLog: "",
});

export const closeRequested = ref(false);

let unlisteners: UnlistenFn[] = [];

export async function initRuntime() {
  state.value = await invoke<RuntimeSnapshot>("get_state");
  unlisteners.push(
    await listen<RuntimeSnapshot>(
      "runtime-state",
      (e) => (state.value = e.payload),
    ),
  );
  unlisteners.push(
    await listen("close-requested", () => (closeRequested.value = true)),
  );
}

export function disposeRuntime() {
  unlisteners.forEach((u) => u());
  unlisteners = [];
}

export const retryStart = () => invoke("retry_start");
export const cancelStart = () => invoke("cancel_start");
export const exitApp = () => invoke("exit_app");
export const hideToTray = () => invoke("hide_to_tray");
export const openLogDir = () => invoke("open_log_dir");
