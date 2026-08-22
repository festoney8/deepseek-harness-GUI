import { ref } from "vue";

export type ToastKind = "info" | "success" | "warning" | "error";

export interface ToastItem {
  id: number;
  kind: ToastKind;
  message: string;
}

const toasts = ref<ToastItem[]>([]);
let nextId = 1;

export function useToast() {
  function dismiss(id: number): void {
    toasts.value = toasts.value.filter((toast) => toast.id !== id);
  }

  function show(kind: ToastKind, message: string): number {
    const id = nextId++;
    toasts.value = [...toasts.value.slice(-2), { id, kind, message }];
    window.setTimeout(() => dismiss(id), kind !== "error" ? 5000 : 10000);
    return id;
  }

  return {
    toasts,
    dismiss,
    show,
    info: (message: string) => show("info", message),
    success: (message: string) => show("success", message),
    warning: (message: string) => show("warning", message),
    error: (message: string) => show("error", message),
  };
}

export function getErrorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === "string" && message) return message;
  }
  return "操作失败，请查看日志";
}
