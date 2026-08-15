import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Phase = "idle" | "installing" | "starting" | "ready" | "failed";

export interface RuntimeSnapshot {
  phase: Phase;
  port: number | null;
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

/** harness 完整命令行输出（stdout+stderr 合并） */
export const output = ref("");

export const closeRequested = ref(false);

const OUTPUT_LIMIT = 1_048_576;

let unlisteners: UnlistenFn[] = [];

export async function initRuntime() {
  // 先注册监听再拉全量，避免拉取窗口内的行丢失
  unlisteners.push(await listen<RuntimeSnapshot>("runtime-state", (e) => onState(e.payload)));
  unlisteners.push(await listen<string>("terminal", (e) => appendOutput(e.payload)));
  unlisteners.push(await listen("close-requested", () => (closeRequested.value = true)));
  state.value = await invoke<RuntimeSnapshot>("get_state");
  output.value = await invoke<string>("get_output");
}

export function disposeRuntime() {
  unlisteners.forEach((u) => u());
  unlisteners = [];
}

async function onState(s: RuntimeSnapshot) {
  if (s.phase === "failed" && state.value.phase === "ready") {
    // Ready 崩溃回失败页时拉一次全量，补齐运行期间未推送的缓冲内容
    output.value = await invoke<string>("get_output");
  }
  state.value = s;
}

function appendOutput(line: string) {
  if (state.value.phase === "ready") return;
  output.value += `${line}\n`;
  if (output.value.length > OUTPUT_LIMIT) {
    output.value = truncateTail(output.value, OUTPUT_LIMIT);
  }
}

/** 保留尾部 limit 个 UTF-16 码元，截断点不落在代理对中间 */
function truncateTail(s: string, limit: number) {
  let start = s.length - limit;
  if (start <= 0) return s;
  const c = s.charCodeAt(start);
  if (c >= 0xdc00 && c <= 0xdfff) start -= 1;
  return s.slice(start);
}

export const checkEnv = () => invoke("check_env");
export const checkVersion = () => invoke("check_version");
export const installDsh = (mirror: boolean) => invoke("install_dsh", { mirror });
export const startServer = () => invoke("start_server");
export const exitApp = () => invoke("exit_app");
export const hideToTray = () => invoke("hide_to_tray");
