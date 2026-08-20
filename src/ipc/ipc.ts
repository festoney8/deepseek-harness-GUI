import { invoke } from "@tauri-apps/api/core";

/** Rust 侧 IpcError 的镜像类型，字段与 ipc.rs 中 camelCase 序列化对齐。 */
export interface IpcError {
  code: string;
  message: string;
}

/** dsh_exited 事件 payload，对应 backend/harness.rs 中 emit 的 JSON。 */
export interface DshExitedPayload {
  exitCode: number | null;
}

/**
 * 将 invoke rejection 归一化为 IpcError。Rust 侧序列化错误会以
 * `{ code, message }` 对象到达；其他非结构化值统一映射为 internal_error。
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

/** 执行自定义 IPC 命令并归一化错误；所有业务命令都经由该入口。 */
export async function invokeIpc<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (error) {
    throw toIpcError(error);
  }
}

export const startDsh = (port: number) => invokeIpc<string>("start_dsh", { port });

export const stopDsh = () => invokeIpc<void>("stop_dsh");

export const connectRemote = (protocol: string, host: string, port: number) =>
  invokeIpc<string>("connect_remote", { protocol, host, port });

export const openLogs = () => invokeIpc<void>("open_logs");

export const hideToTray = () => invokeIpc<void>("hide_to_tray");
