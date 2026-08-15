<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import TitleBar from "./components/TitleBar.vue";
import StatusView from "./components/StatusView.vue";
import WebUiView from "./components/WebUiView.vue";
import { closeRequested, disposeRuntime, exitApp, hideToTray, initRuntime, state } from "./composables/useRuntime";
import { disposeTheme, initTheme } from "./composables/useTheme";

function hideInTray() {
  hideToTray();
  closeRequested.value = false;
}

onMounted(initRuntime);
onMounted(initTheme);
onUnmounted(disposeRuntime);
onUnmounted(disposeTheme);
</script>

<template>
  <div class="flex h-full flex-col bg-white dark:bg-slate-900">
    <TitleBar />
    <main class="min-h-0 flex-1">
      <WebUiView v-if="state.phase === 'ready' && state.port" :port="state.port" />
      <StatusView v-else :state="state" />
    </main>

    <div v-if="closeRequested" class="fixed inset-0 z-50 flex items-center justify-center bg-black/30">
      <div class="w-80 rounded-lg bg-white p-5 shadow-xl dark:bg-slate-800">
        <h2 class="text-sm font-semibold dark:text-slate-100">退出 DeepSeek Harness？</h2>
        <div class="mt-12 flex justify-end gap-2">
          <button
            class="cursor-pointer rounded-md border border-slate-300 px-3 py-1 text-sm text-slate-600 transition hover:bg-slate-100 dark:border-slate-600 dark:text-slate-300 dark:hover:bg-slate-700"
            @click="closeRequested = false"
          >
            取消
          </button>
          <button
            class="cursor-pointer rounded-md border border-slate-300 px-3 py-1 text-sm text-slate-600 transition hover:bg-slate-100 dark:border-slate-600 dark:text-slate-300 dark:hover:bg-slate-700"
            @click="hideInTray"
          >
            最小化到托盘
          </button>
          <button
            class="cursor-pointer rounded-md border border-transparent bg-red-500 px-3 py-1 text-sm text-white transition hover:bg-red-600"
            @click="exitApp"
          >
            退出
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
