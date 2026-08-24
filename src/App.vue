<template>
  <div id="background" class="flex h-screen min-h-0 flex-col overflow-hidden bg-base-200 text-base-content">
    <AppNavbar />
    <main class="min-h-0 flex-1 overflow-auto">
      <HomeView />
    </main>
  </div>
  <ToastViewport />
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, watch } from "vue";
import HomeView from "./views/HomeView.vue";
import AppNavbar from "./components/layout/AppNavbar.vue";
import ToastViewport from "./components/feedback/ToastViewport.vue";
import { useDshStore } from "./stores/dsh";
import { useEnvStore } from "./stores/env";
import { getErrorMessage, useToast } from "./composables/useToast";
import { useTheme } from "./composables/useTheme";
import { logger } from "./utils/log";

const dsh = useDshStore();
const env = useEnvStore();
const toast = useToast();
useTheme();

let stopUnexpectedExitWatch: (() => void) | undefined;

document.addEventListener("keydown", (e) => {
  const key = e.key.toLowerCase();
  if (key === "f5" || ((e.ctrlKey || e.metaKey) && key === "r")) {
    e.preventDefault();
  }
});
document.addEventListener(
  "contextmenu",
  (e) => {
    e.preventDefault();
  },
  false,
);

onMounted(async () => {
  stopUnexpectedExitWatch = watch(
    () => dsh.unexpectedExit,
    (unexpected) => {
      if (!unexpected) return;
      toast.error("DSH 进程已异常退出，请检查日志");
      dsh.clearUnexpectedExit();
    },
  );
  try {
    await dsh.bindEvents();
  } catch (error) {
    logger.error("dsh", "注册 dsh_exited 监听失败，将无法感知 DSH 异常退出:", error);
    toast.error(getErrorMessage(error));
  }
  void env.refreshAllVersions();
  void env.getLatestAppVer();
  void env.getAppVer();
});

onBeforeUnmount(() => {
  stopUnexpectedExitWatch?.();
  dsh.dispose();
});
</script>

<style lang="scss" scoped>
#background {
  background: radial-gradient(circle at top, #e8f1ff 0, #fcfcff 42%, #faf7ff 100%);
}
[data-theme="night"] {
  #background {
    background:
      radial-gradient(circle at 10% 15%, rgba(30, 64, 175, 0.28) 0%, rgba(30, 64, 175, 0.12) 25%, transparent 55%),
      radial-gradient(circle at 90% 10%, rgba(79, 70, 229, 0.22) 0%, rgba(79, 70, 229, 0.08) 30%, transparent 60%),
      radial-gradient(circle at 75% 80%, rgba(14, 116, 144, 0.18) 0%, transparent 55%),
      radial-gradient(circle at 25% 90%, rgba(67, 56, 202, 0.14) 0%, transparent 50%),
      linear-gradient(135deg, #020617 0%, #0b1120 30%, #111827 60%, #0f172a 100%);
  }
}
</style>
