import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { startDsh, stopDsh, type DshExitedPayload, type IpcError } from "../ipc/ipc";

/** dsh 生命周期阶段 */
export type DshPhase = "stopped" | "starting" | "running" | "stopping";

/**
 * 镜像后端 HarnessPhase 的 dsh 生命周期状态（DESIGN.md §7.1）
 * Rust 侧是权威状态机，前端只做镜像与错误码分支
 */
export const useDshStore = defineStore("dsh", () => {
  /** dsh 生命周期阶段 */
  const phase = ref<DshPhase>("stopped");
  /** 当前 WebUI 端口，仅运行时有值 */
  const currPort = ref<number | null>(null);
  /** 当前 WebUI 地址，仅运行时有值 */
  const address = ref<string | null>(null);
  /** 最近一次 IPC 错误 */
  const lastError = ref<IpcError | null>(null);
  /** 最近一次 dsh_exited 是否为非主动退出 */
  const unexpectedExit = ref(false);

  /** 是否运行中 */
  const isRunning = computed(() => phase.value === "running");
  /** 是否处于启动/停止过渡中 */
  const isBusy = computed(() => phase.value === "starting" || phase.value === "stopping");
  /** 当前是否可启动 */
  const canStart = computed(() => phase.value === "stopped");

  /** 幂等去重用的挂起注册 Promise */
  let bindPromise: Promise<UnlistenFn> | null = null;
  /** dsh_exited 监听注销函数 */
  let unlisten: UnlistenFn | null = null;

  /**
   * 启动 dsh 并返回当前使用的端口号；
   * 若已有生命周期操作在进行则抛出 IpcError（operation_in_progress）
   */
  async function start(port: number): Promise<string> {
    const previousPhase = phase.value;
    if (previousPhase === "starting" || previousPhase === "stopping") {
      const error: IpcError = {
        code: "operation_in_progress",
        message: "已有 dsh 生命周期操作正在进行",
      };
      lastError.value = error;
      throw error;
    }
    phase.value = "starting";
    lastError.value = null;
    try {
      const startedAddress = await startDsh(port);
      phase.value = "running";
      currPort.value = port;
      address.value = startedAddress;
      unexpectedExit.value = false;
      return startedAddress;
    } catch (error) {
      const ipcError = error as IpcError;
      if (ipcError.code === "dsh_already_running") {
        phase.value = "running";
        lastError.value = ipcError;
        throw error;
      }
      if (ipcError.code === "operation_in_progress") {
        phase.value = previousPhase;
      } else {
        phase.value = "stopped";
        currPort.value = null;
        address.value = null;
      }
      lastError.value = ipcError;
      throw error;
    }
  }

  /** 停止 dsh 并清理进程相关状态 */
  async function stop(): Promise<void> {
    const previousPhase = phase.value;
    if (previousPhase === "stopped") return;
    if (previousPhase === "starting" || previousPhase === "stopping") {
      const error: IpcError = {
        code: "operation_in_progress",
        message: "已有 dsh 生命周期操作正在进行",
      };
      lastError.value = error;
      throw error;
    }
    phase.value = "stopping";
    lastError.value = null;
    try {
      await stopDsh();
      phase.value = "stopped";
      currPort.value = null;
      address.value = null;
    } catch (error) {
      const ipcError = error as IpcError;
      if (ipcError.code === "process_not_running") {
        phase.value = "stopped";
        currPort.value = null;
        address.value = null;
      } else {
        phase.value = previousPhase;
      }
      lastError.value = ipcError;
      throw error;
    }
  }

  /**
   * 重启 dsh：先停止当前实例（已处于 Stopped 或无进程时视为停止完成），
   * 再以同一端口启动。返回后端返回的 WebUI 地址
   */
  async function restart(port: number): Promise<string> {
    try {
      await stop();
    } catch (error) {
      const ipcError = error as IpcError;
      if (ipcError.code !== "process_not_running") {
        throw error;
      }
    }
    return start(port);
  }

  /**
   * 注册 dsh_exited 监听。幂等；并发调用共享同一个挂起的注册 promise，
   * 确保只注册一次。返回 unlisten，供 dispose 或应用卸载时清理
   * 只在 App.vue 挂载时调用一次，避免组件生命周期造成事件丢失
   */
  async function bindEvents(): Promise<UnlistenFn> {
    if (unlisten) return unlisten;
    bindPromise ??= listen<DshExitedPayload>("dsh_exited", () => {
      phase.value = "stopped";
      currPort.value = null;
      address.value = null;
      unexpectedExit.value = true;
    });
    try {
      unlisten = await bindPromise;
    } finally {
      bindPromise = null;
    }
    return unlisten;
  }

  /** 清除已经展示过的异常退出提醒 */
  function clearUnexpectedExit(): void {
    unexpectedExit.value = false;
  }

  /** 注销并清空 dsh_exited 监听引用 */
  function dispose(): void {
    unlisten?.();
    unlisten = null;
  }

  return {
    phase,
    currPort,
    address,
    lastError,
    unexpectedExit,
    isRunning,
    isBusy,
    canStart,
    start,
    stop,
    restart,
    bindEvents,
    clearUnexpectedExit,
    dispose,
  };
});
