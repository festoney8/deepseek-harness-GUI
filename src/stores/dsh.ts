import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { startDsh, stopDsh, type DshExitedPayload, type IpcError } from "../utils/ipc";

export type DshPhase = "stopped" | "starting" | "running" | "stopping";

/**
 * 镜像后端 HarnessPhase 的 dsh 生命周期状态（DESIGN.md §7.1）。
 * Rust 侧是权威状态机，前端只做镜像与错误码分支。
 */
export const useDshStore = defineStore("dsh", () => {
  const phase = ref<DshPhase>("stopped");
  const address = ref<string | null>(null);
  const lastError = ref<IpcError | null>(null);

  const isRunning = computed(() => phase.value === "running");
  const isBusy = computed(() => phase.value === "starting" || phase.value === "stopping");
  const canStart = computed(() => phase.value === "stopped");

  let bindPromise: Promise<UnlistenFn> | null = null;
  let unlisten: UnlistenFn | null = null;

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
      const result = await startDsh(port);
      phase.value = "running";
      address.value = result;
      return result;
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
        address.value = null;
      }
      lastError.value = ipcError;
      throw error;
    }
  }

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
      address.value = null;
    } catch (error) {
      const ipcError = error as IpcError;
      if (ipcError.code === "process_not_running") {
        phase.value = "stopped";
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
   * 再以同一端口启动。返回新的 WebUI 地址。
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
   * 确保只注册一次。返回 unlisten，供 dispose 或应用卸载时清理。
   * 只在 App.vue 挂载时调用一次，避免组件生命周期造成事件丢失。
   */
  async function bindEvents(): Promise<UnlistenFn> {
    if (unlisten) return unlisten;
    bindPromise ??= listen<DshExitedPayload>("dsh_exited", () => {
      phase.value = "stopped";
      address.value = null;
    });
    try {
      unlisten = await bindPromise;
    } finally {
      bindPromise = null;
    }
    return unlisten;
  }

  function dispose(): void {
    unlisten?.();
    unlisten = null;
  }

  return {
    phase,
    address,
    lastError,
    isRunning,
    isBusy,
    canStart,
    start,
    stop,
    restart,
    bindEvents,
    dispose,
  };
});
