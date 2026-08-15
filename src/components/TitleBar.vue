<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import logoMiniUrl from "../assets/logo_mini.png";

const win = getCurrentWindow();
const maximized = ref(false);
let unlisten: (() => void) | undefined;

async function syncMaximized() {
  maximized.value = await win.isMaximized();
}

onMounted(async () => {
  await syncMaximized();
  unlisten = await win.onResized(syncMaximized);
});

onUnmounted(() => unlisten?.());
</script>

<template>
  <header
    class="flex h-8 shrink-0 items-stretch border-b border-slate-200 bg-slate-50 dark:border-slate-700 dark:bg-slate-800"
    data-tauri-drag-region
  >
    <div class="flex flex-1 items-center gap-2 px-3" data-tauri-drag-region>
      <img :src="logoMiniUrl" alt="logo" class="h-5 w-5 shrink-0 rounded" draggable="false" />
      <span class="text-xs font-medium text-slate-600 dark:text-slate-300">DeepSeek Harness GUI</span>
    </div>
    <button
      class="flex w-11 items-center justify-center text-slate-500 hover:bg-slate-200 dark:text-slate-400 dark:hover:bg-slate-700"
      title="最小化"
      @click="win.minimize()"
    >
      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
        <path d="M0 5h10" stroke="currentColor" stroke-width="1" />
      </svg>
    </button>
    <button
      class="flex w-11 items-center justify-center text-slate-500 hover:bg-slate-200 dark:text-slate-400 dark:hover:bg-slate-700"
      :title="maximized ? '还原' : '最大化'"
      @click="win.toggleMaximize()"
    >
      <svg v-if="!maximized" width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
        <rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" />
      </svg>
      <svg v-else width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
        <path d="M0.5 3.5h6v6h-6zM3.5 0.5h6v6" fill="none" stroke="currentColor" transform="translate(0 0)" />
      </svg>
    </button>
    <button
      class="flex w-11 items-center justify-center text-slate-500 hover:bg-red-500 hover:text-white dark:text-slate-400 dark:hover:bg-red-500 dark:hover:text-white"
      title="关闭"
      @click="win.close()"
    >
      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
        <path d="M0 0l10 10M10 0L0 10" stroke="currentColor" stroke-width="1" />
      </svg>
    </button>
  </header>
</template>
