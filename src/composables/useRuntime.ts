import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Phase = "idle" | "installing" | "starting" | "ready" | "failed";

export interface RuntimeSnapshot {
  phase: Phase;
  port: number | null;
  /** 完整访问地址（如 http://127.0.0.1:3080/），ready 时由后端拼好 */
  url: string | null;
  detail: string;
  elapsed: number | null;
  /** 格子一：node / npm 版本（null = 未检测到） */
  node: string | null;
  npm: string | null;
  /** 格子二：远端 / 本地 dsh 版本（local null = 未安装；remote null = 获取失败） */
  remote: string | null;
  /** 镜像源查询到的远端版本（null = 获取失败，仅用于展示） */
  remoteMirror: string | null;
  local: string | null;
  versionError: boolean;
  /** 版本检查是否已完成（false = 尚未检查，显示“检查中”） */
  versionChecked: boolean;
}

export const state = ref<RuntimeSnapshot>({
  phase: "idle",
  port: null,
  url: null,
  detail: "",
  elapsed: null,
  node: null,
  npm: null,
  remote: null,
  remoteMirror: null,
  local: null,
  versionError: false,
  versionChecked: false,
});

export const closeRequested = ref(false);

let unlisteners: UnlistenFn[] = [];

export async function initRuntime() {
  // 先注册监听再拉全量，避免拉取窗口内的行丢失
  unlisteners.push(await listen<RuntimeSnapshot>("runtime-state", (e) => onState(e.payload)));
  unlisteners.push(await listen("close-requested", () => (closeRequested.value = true)));
  state.value = await invoke<RuntimeSnapshot>("get_state");
}

export function disposeRuntime() {
  unlisteners.forEach((u) => u());
  unlisteners = [];
}

function onState(s: RuntimeSnapshot) {
  state.value = s;
}

export const checkEnv = () => invoke("check_env");
export const checkVersion = () => invoke("check_version");
export const installDsh = (mirror: boolean) => invoke("install_dsh", { mirror });
export const startServer = (host: string, port: number) => invoke("start_server", { host, port });
export const openLogDir = () => invoke("open_log_dir");
export const exitApp = () => invoke("exit_app");
export const hideToTray = () => invoke("hide_to_tray");
