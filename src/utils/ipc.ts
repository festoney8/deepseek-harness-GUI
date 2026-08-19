import { invokeIpc } from "../ipc/ipc";

export type { DshExitedPayload, IpcError } from "../ipc/ipc";

export const startDsh = (port: number) => invokeIpc<string>("start_dsh", { port });

export const stopDsh = () => invokeIpc<void>("stop_dsh");

export const connectRemote = (protocol: string, host: string, port: number) =>
  invokeIpc<string>("connect_remote", { protocol, host, port });

export const openLogs = () => invokeIpc<void>("open_logs");

export const hideToTray = () => invokeIpc<void>("hide_to_tray");
