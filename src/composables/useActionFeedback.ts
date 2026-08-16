import { ref, watch } from "vue";
import { state } from "./useRuntime";

export const actionError = ref<string | null>(null);

let actionPending = false;
let lastFailureKey = "";

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
  lastFailureKey = "";
  actionError.value = null;
}

export function reportActionError(error: unknown) {
  actionPending = false;
  actionError.value = errorMessage(error);
}

export function clearActionError() {
  actionPending = false;
  lastFailureKey = "";
  actionError.value = null;
}

export function useActionFeedback() {
  watch(
    state,
    (snapshot) => {
      if (snapshot.phase === "failed" && snapshot.detail && actionPending) {
        const failureKey = `${snapshot.phase}:${snapshot.detail}`;
        if (failureKey !== lastFailureKey) {
          lastFailureKey = failureKey;
          actionError.value = snapshot.detail;
        }
        actionPending = false;
      }
    },
    { immediate: true },
  );

  return { actionError, clearActionError };
}
