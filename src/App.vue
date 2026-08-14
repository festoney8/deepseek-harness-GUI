<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import TitleBar from "./components/TitleBar.vue";
import StatusView from "./components/StatusView.vue";
import WebUiView from "./components/WebUiView.vue";
import {
  closeRequested,
  disposeRuntime,
  exitApp,
  hideToTray,
  initRuntime,
  state,
} from "./composables/useRuntime";

function hideInTray() {
  hideToTray();
  closeRequested.value = false;
}

onMounted(initRuntime);
onUnmounted(disposeRuntime);
</script>

<template>
  <div class="flex h-full flex-col bg-white">
    <TitleBar />
    <main class="min-h-0 flex-1">
      <WebUiView
        v-if="state.phase === 'ready' && state.port"
        :port="state.port"
      />
      <StatusView v-else :state="state" />
    </main>

    <div
      v-if="closeRequested"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
    >
      <div class="w-80 rounded-lg bg-white p-5 shadow-xl">
        <h2 class="text-sm font-semibold">退出 DeepSeek Harness？</h2>
        <p class="mt-1 text-xs leading-5 text-slate-500">
          退出将同时停止 DeepSeek Harness
          服务；最小化到托盘可让服务在后台继续运行。
        </p>
        <div class="mt-4 flex justify-end gap-2">
          <button class="dialog-btn" @click="closeRequested = false">
            取消
          </button>
          <button class="dialog-btn" @click="hideInTray">最小化到托盘</button>
          <button class="dialog-btn danger" @click="exitApp">退出</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-btn {
  border-radius: 0.375rem;
  border: 1px solid #cbd5e1;
  padding: 0.25rem 0.75rem;
  font-size: 0.875rem;
  color: #475569;
}
.dialog-btn:hover {
  background-color: #f1f5f9;
}
.dialog-btn.danger {
  border-color: transparent;
  background-color: #ef4444;
  color: #fff;
}
.dialog-btn.danger:hover {
  background-color: #dc2626;
}
</style>
