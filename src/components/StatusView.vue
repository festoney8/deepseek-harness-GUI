<script setup lang="ts">
import { computed } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { RuntimeSnapshot } from "../composables/useRuntime";
import {
  cancelStart,
  exitApp,
  openLogDir,
  retryStart,
} from "../composables/useRuntime";

const props = defineProps<{ state: RuntimeSnapshot }>();

const busy = computed(
  () =>
    props.state.phase === "checkingNode" ||
    props.state.phase === "findingPort" ||
    props.state.phase === "starting",
);
const showLog = computed(() => props.state.lastLog.length > 0);
const elapsed = computed(() => {
  const s = props.state.elapsed;
  return s == null ? "" : `${Math.floor(s / 60)} 分 ${s % 60} 秒`;
});
</script>

<template>
  <div class="flex h-full flex-col items-center justify-center gap-5 px-8">
    <div
      v-if="busy"
      class="h-10 w-10 animate-spin rounded-full border-4 border-slate-200 border-t-blue-500"
    />

    <div class="text-center">
      <h1 class="text-lg font-semibold">{{ state.detail }}</h1>
      <p
        v-if="state.phase === 'starting' && state.port"
        class="mt-1 text-sm text-slate-500"
      >
        端口 {{ state.port }} · 已等待 {{ elapsed }}
      </p>
    </div>

    <pre
      v-if="showLog"
      class="max-h-32 w-full max-w-xl overflow-auto whitespace-pre-wrap rounded-md bg-slate-100 p-3 font-mono text-xs text-slate-600"
      >{{ state.lastLog }}</pre>

    <div v-if="state.phase === 'starting'" class="flex gap-2">
      <button class="btn-secondary" @click="cancelStart">取消启动</button>
    </div>

    <div
      v-else-if="state.phase === 'envMissing'"
      class="flex flex-col items-center gap-3"
    >
      <p class="max-w-md text-center text-sm text-slate-500">
        本应用需要 Node.js 环境。请先安装 Node.js（LTS
        版本即可），安装完成后点击重新检测。
      </p>
      <div class="flex gap-2">
        <button
          class="btn-primary"
          @click="openUrl('https://nodejs.org/zh-cn/download')"
        >
          打开 Node.js 官网
        </button>
        <button class="btn-secondary" @click="retryStart">重新检测</button>
        <button class="btn-secondary" @click="exitApp">退出</button>
      </div>
    </div>

    <div
      v-else-if="state.phase === 'failed'"
      class="flex flex-col items-center gap-3"
    >
      <p class="max-w-md text-center text-sm text-slate-500">
        {{ state.detail }}
      </p>
      <div class="flex gap-2">
        <button class="btn-primary" @click="retryStart">重试</button>
        <button class="btn-secondary" @click="openLogDir">打开日志目录</button>
        <button class="btn-secondary" @click="exitApp">退出</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.btn-primary {
  border-radius: 0.375rem;
  background-color: #3b82f6;
  padding: 0.375rem 1rem;
  font-size: 0.875rem;
  color: #fff;
}
.btn-primary:hover {
  background-color: #2563eb;
}
.btn-secondary {
  border-radius: 0.375rem;
  border: 1px solid #cbd5e1;
  padding: 0.375rem 1rem;
  font-size: 0.875rem;
  color: #475569;
}
.btn-secondary:hover {
  background-color: #f1f5f9;
}
</style>
