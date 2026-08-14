<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { RuntimeSnapshot } from "../composables/useRuntime";
import {
  cancelStart,
  checkEnv,
  checkVersion,
  installDsh,
  output,
  startServer,
} from "../composables/useRuntime";

const props = defineProps<{ state: RuntimeSnapshot }>();
const logRef = ref<HTMLElement | null>(null);

const installing = computed(() => props.state.phase === "installing");
const starting = computed(() => props.state.phase === "starting");
const failed = computed(() => props.state.phase === "failed");
const busy = computed(() => installing.value || starting.value);
const needInstall = computed(
  () => !props.state.versionError && props.state.local !== props.state.remote,
);
/** 环境都正常（node/npm 就绪、dsh 已安装且版本一致）时才允许启动 */
const canStart = computed(
  () =>
    !busy.value &&
    props.state.node != null &&
    props.state.npm != null &&
    props.state.local != null &&
    props.state.local === props.state.remote,
);
const startHint = computed(() => {
  if (busy.value) return "";
  if (props.state.node == null || props.state.npm == null)
    return "请先安装 Node.js 环境";
  if (props.state.local == null) return "请先安装 DeepSeek Harness";
  if (props.state.local !== props.state.remote)
    return "请先更新 DeepSeek Harness";
  return "";
});
const elapsed = computed(() => {
  const s = props.state.elapsed;
  return s == null ? "" : `${Math.floor(s / 60)} 分 ${s % 60} 秒`;
});

function onStartClick() {
  if (starting.value) {
    cancelStart();
  } else {
    startServer();
  }
}

// 动作执行中随输出自动滚动到底部；失败后停止滚动，方便用户回看完整输出
watch(output, async () => {
  if (!failed.value) {
    await nextTick();
    const el = logRef.value;
    if (el) el.scrollTop = el.scrollHeight;
  }
});
</script>

<template>
  <div class="flex h-full flex-col justify-center gap-5 px-8 py-6">
    <!-- 两格状态面板 -->
    <div class="grid shrink-0 grid-cols-2 gap-4">
      <!-- 格子一：Node.js 环境 -->
      <section class="panel">
        <h2 class="text-base font-semibold text-slate-700">Node.js 环境</h2>
        <dl class="mt-3 space-y-2 text-base">
          <div class="flex justify-between gap-2">
            <dt class="text-slate-500">node</dt>
            <dd
              class="font-mono font-semibold"
              :class="state.node ? '' : 'text-red-500'"
            >
              {{ state.node ?? "未检测到" }}
            </dd>
          </div>
          <div class="flex justify-between gap-2">
            <dt class="text-slate-500">npm</dt>
            <dd
              class="font-mono font-semibold"
              :class="state.npm ? '' : 'text-red-500'"
            >
              {{ state.npm ?? "未检测到" }}
            </dd>
          </div>
        </dl>
        <div class="mt-4 flex flex-wrap gap-2">
          <button
            v-if="!state.node"
            class="btn-primary"
            @click="openUrl('https://nodejs.org/en/download')"
          >
            打开下载页面
          </button>
          <button class="btn-secondary" :disabled="busy" @click="checkEnv">
            重新检查
          </button>
        </div>
      </section>

      <!-- 格子二：DeepSeek Harness 版本 -->
      <section class="panel">
        <h2 class="text-base font-semibold text-slate-700">DeepSeek Harness</h2>
        <dl class="mt-3 space-y-2 text-base">
          <div class="flex justify-between gap-2">
            <dt class="text-slate-500">远端版本</dt>
            <dd
              class="font-mono font-semibold"
              :class="state.remote ? '' : 'text-red-500'"
            >
              {{ state.remote ?? (state.versionError ? "获取失败" : "…") }}
            </dd>
          </div>
          <div class="flex justify-between gap-2">
            <dt class="text-slate-500">本地版本</dt>
            <dd
              class="font-mono font-semibold"
              :class="state.local ? '' : 'text-slate-400'"
            >
              {{ state.local ?? "未安装" }}
            </dd>
          </div>
        </dl>
        <div class="mt-4 flex flex-wrap gap-2">
          <button
            v-if="state.versionError"
            class="btn-secondary"
            :disabled="busy"
            @click="checkVersion"
          >
            重新检查
          </button>
          <button
            v-else-if="needInstall"
            class="btn-primary"
            :disabled="busy"
            @click="installDsh"
          >
            {{ installing ? "安装中…" : state.local ? "更新" : "安装" }}
          </button>
          <span v-else class="self-center text-xs text-emerald-600"
            >已是最新版本</span
          >
        </div>
      </section>
    </div>

    <!-- 巨大启动按钮 -->
    <div class="flex shrink-0 flex-col items-center gap-2">
      <button class="start-btn" :disabled="!canStart" @click="onStartClick">
        {{ starting ? "正在启动…（点击取消）" : "启动 DeepSeek Harness" }}
      </button>
      <p v-if="starting" class="text-xs text-slate-500">已等待 {{ elapsed }}</p>
      <p v-if="failed" class="text-xs text-red-500">{{ state.detail }}</p>
      <p v-else-if="startHint" class="text-xs text-slate-400">
        {{ startHint }}
      </p>
    </div>

    <!-- 底部命令行输出 -->
    <div class="flex min-h-0 flex-1 flex-col">
      <div class="mb-1 text-xs font-semibold text-slate-500">命令行输出</div>
      <pre
        ref="logRef"
        class="log-panel min-h-0 select-text overflow-auto whitespace-pre-wrap rounded-md bg-slate-100 p-3 font-mono text-xs leading-5 text-slate-700"
        >{{ output }}</pre>
    </div>
  </div>
</template>

<style scoped>
.log-panel {
  height: min(30vh, 350px);
}
.panel {
  border: 1px solid #e2e8f0;
  border-radius: 0.5rem;
  background-color: #fff;
  padding: 1rem;
}
.start-btn {
  min-width: 340px;
  border-radius: 0.75rem;
  background-color: #3b82f6;
  padding: 1rem 3rem;
  font-size: 1.25rem;
  font-weight: 600;
  color: #fff;
}
.start-btn:hover:not(:disabled) {
  background-color: #2563eb;
}
.start-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn-primary {
  border-radius: 0.375rem;
  background-color: #3b82f6;
  padding: 0.375rem 1rem;
  font-size: 0.875rem;
  color: #fff;
}
.btn-primary:hover:not(:disabled) {
  background-color: #2563eb;
}
.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn-secondary {
  border-radius: 0.375rem;
  border: 1px solid #cbd5e1;
  padding: 0.375rem 1rem;
  font-size: 0.875rem;
  color: #475569;
}
.btn-secondary:hover:not(:disabled) {
  background-color: #f1f5f9;
}
.btn-secondary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
