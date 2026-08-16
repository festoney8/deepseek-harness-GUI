import { ref, watch } from "vue";
import { state } from "./useRuntime";

export const actionError = ref<string | null>(null);

let actionPending = false;

function errorMessage(error: unknown) {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object") {
    try {
      return JSON.stringify(error);
    } catch {
      return "操作失败，请查看日志。";
    }
  }
  return "操作失败，请查看日志。";
}

export function beginAction() {
  actionPending = true;
  actionError.value = null;
}

export function reportActionError(error: unknown) {
  actionPending = false;
  actionError.value = errorMessage(error);
}

export function clearActionError() {
  actionPending = false;
  actionError.value = null;
}

export function useActionFeedback() {
  watch(
    state,
    (snapshot) => {
      if (snapshot.phase === "failed" && snapshot.detail && actionPending) {
        actionError.value = snapshot.detail;
        actionPending = false;
      } else if (actionPending && (snapshot.phase === "idle" || snapshot.phase === "ready")) {
        // 动作成功完成（安装 → idle，启动 → ready），清理待处理标记，
        // 避免后续无关的 failed 事件误触发 toast。
        actionPending = false;
      }
    },
    { immediate: true },
  );

  return { actionError, clearActionError };
}
