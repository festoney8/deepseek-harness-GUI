import { invoke } from "@tauri-apps/api/core";

/** Rust 侧 IpcError 的镜像类型，字段与 ipc.rs 中 camelCase 序列化对齐 */
export interface IpcError {
  code: string;
  message: string;
}

/** dsh_exited 事件 payload，对应 backend/harness.rs 中 emit 的 JSON */
export interface DshExitedPayload {
  exitCode: number | null;
}

/** Rust 侧 child WebView 标签信息 */
export interface WebviewTab {
  label: string;
  url: string;
  displayName: string;
}

/**
 * 将 invoke rejection 归一化为 IpcError。Rust 侧序列化错误会以
 * `{ code, message }` 对象到达；其他非结构化值统一映射为 internal_error
 */
function toIpcError(payload: unknown): IpcError {
  if (
    typeof payload === "object" &&
    payload !== null &&
    typeof (payload as { code?: unknown }).code === "string" &&
    typeof (payload as { message?: unknown }).message === "string"
  ) {
    return payload as IpcError;
  }
  return { code: "internal_error", message: "内部错误，请查看日志" };
}

/** 前端 store 有专门处理分支的预期错误码，失败时按 warn 记录 */
const EXPECTED_IPC_CODES = new Set(["operation_in_progress", "process_not_running", "dsh_already_running"]);

/** 执行自定义 IPC 命令并归一化错误；所有业务命令都经由该入口 */
export async function invokeIpc<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (error) {
    const ipcError = toIpcError(error);
    if (EXPECTED_IPC_CODES.has(ipcError.code)) {
      console.warn("ipc", `${cmd} 预期内失败:`, ipcError.code, ipcError.message);
    } else {
      console.error("ipc", `${cmd} 调用失败:`, ipcError.code, ipcError.message);
    }
    if (ipcError.code === "internal_error") {
      console.error("ipc", `${cmd} 原始错误 payload:`, error);
    }
    throw ipcError;
  }
}

/** 启动 dsh，返回 WebUI 地址 */
export const startDsh = (port: number) => invokeIpc<string>("start_dsh", { port });

/** 停止 dsh */
export const stopDsh = () => invokeIpc<void>("stop_dsh");

/** 探测并返回规范化远程地址 */
export const connectRemote = (protocol: string, host: string, port: number) =>
  invokeIpc<string>("connect_remote", { protocol, host, port });

/** 打开本次日志目录 */
export const openLogs = () => invokeIpc<void>("open_logs");

/** 隐藏到托盘 */
export const hideToTray = () => invokeIpc<void>("hide_to_tray");

/** 创建或激活一个 DSH child WebView 标签 */
export const createWindowWithUrl = (url: string) => invokeIpc<WebviewTab>("create_webview_with_url", { url });

/** 激活一个 DSH child WebView 标签 */
export const activateWebviewTab = (label: string) => invokeIpc<void>("activate_webview_tab", { label });

/** 关闭一个 DSH child WebView 标签 */
export const closeWebviewTab = (label: string) => invokeIpc<void>("close_webview_tab", { label });

/** 隐藏所有 DSH child WebView，显示首页 */
export const hideAllWebviewTabs = () => invokeIpc<void>("hide_all_webview_tabs");
