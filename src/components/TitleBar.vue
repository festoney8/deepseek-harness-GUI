<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import IconMinimize from "~icons/mingcute/minimize-fill";
import IconMaximize from "~icons/mdi/maximize";
import IconClose from "~icons/mingcute/close-line";
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
  <header class="border-base-300 bg-base-200 flex h-8 shrink-0 items-stretch border-b" data-tauri-drag-region>
    <div class="flex flex-1 items-center gap-2 px-3" data-tauri-drag-region>
      <img :src="logoMiniUrl" alt="logo" class="h-5 w-5 shrink-0 rounded" draggable="false" />
      <span class="text-base-content/70 text-xs font-medium">DeepSeek Harness GUI</span>
    </div>

    <button
      type="button"
      class="btn btn-ghost btn-square btn-xs text-base-content/60 hover:bg-base-300 h-8 w-11 rounded-none"
      title="最小化"
      aria-label="最小化"
      @click="win.minimize()"
    >
      <IconMinimize class="h-4 w-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      class="btn btn-ghost btn-square btn-xs text-base-content/60 hover:bg-base-300 h-8 w-11 rounded-none"
      :title="maximized ? '还原' : '最大化'"
      :aria-label="maximized ? '还原' : '最大化'"
      @click="win.toggleMaximize()"
    >
      <IconMaximize class="h-4 w-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      class="btn btn-ghost btn-square btn-xs text-base-content/60 hover:bg-error hover:text-error-content h-8 w-11 rounded-none"
      title="关闭"
      aria-label="关闭"
      @click="win.close()"
    >
      <IconClose class="h-4 w-4" aria-hidden="true" />
    </button>
  </header>
</template>
