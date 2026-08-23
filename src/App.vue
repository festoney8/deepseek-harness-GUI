<template>
  <div class="flex h-screen min-h-0 flex-col overflow-hidden bg-base-200 text-base-content">
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
import { useThemeStore } from "./stores/theme";
import { getErrorMessage, useToast } from "./composables/useToast";
import { useTheme } from "./composables/useTheme";

const dsh = useDshStore();
const env = useEnvStore();
const theme = useThemeStore();
const toast = useToast();
useTheme();

let stopUnexpectedExitWatch: (() => void) | undefined;

onMounted(async () => {
  theme.start();
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
