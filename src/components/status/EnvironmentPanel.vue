<script setup lang="ts">
import { computed, ref } from "vue";
import type { RuntimeSnapshot } from "../../composables/useRuntime";
import { checkEnv, checkVersion } from "../../composables/useRuntime";

const { state } = defineProps<{ state: RuntimeSnapshot }>();

const busy = computed(() => state.phase === "installing" || state.phase === "starting");

/** 版本行的三态（有值/失败/检查中）显示与样式；highlighted 时对已有版本改用琥珀色提示 */
const versionRow = (label: string, value: string | null, failed: boolean, highlighted = false) => ({
  label,
  value: value ?? (failed ? "获取失败" : "检查中…"),
  valueClass: value
    ? highlighted
      ? "text-amber-600 dark:text-amber-400"
      : "text-slate-900 dark:text-slate-100"
    : failed
      ? "text-rose-600 dark:text-rose-400"
      : "text-amber-600 dark:text-amber-400",
  dotClass: value ? "bg-emerald-500" : failed ? "bg-rose-500" : "bg-amber-400",
});
const statusRows = computed(() => [
  {
    label: "node 版本（推荐 v24 及以上）",
    value: state.node ?? "未检测到",
    valueClass: state.node ? "text-slate-900 dark:text-slate-100" : "text-rose-600 dark:text-rose-400",
    dotClass: state.node ? "bg-emerald-500" : "bg-rose-500",
  },
  {
    label: "npm 版本",
    value: state.npm ?? "未检测到",
    valueClass: state.npm ? "text-slate-900 dark:text-slate-100" : "text-rose-600 dark:text-rose-400",
    dotClass: state.npm ? "bg-emerald-500" : "bg-rose-500",
  },
  versionRow(
    "最新 DSH 版本（官方源）",
    state.remote,
    state.versionError,
    state.local !== null && state.remote !== state.local,
  ),
  versionRow(
    "最新 DSH 版本（镜像源）",
    state.remoteMirror,
    state.versionChecked,
    state.local !== null && state.remoteMirror !== state.local,
  ),
  {
    label: "本地 DSH 版本",
    value: state.local ?? (state.versionChecked ? "未安装" : "检查中…"),
    valueClass: state.local ? "text-slate-900 dark:text-slate-100" : "text-amber-600 dark:text-amber-400",
    dotClass: state.local ? "bg-emerald-500" : "bg-amber-400",
  },
]);

/** 重新检查按钮 icon 的旋转动画进行中（转两圈后由 animationend 清除） */
const checking = ref(false);

function recheckEnvironment() {
  if (checking.value) return;
  checking.value = true;
  void Promise.allSettled([checkEnv(), checkVersion()]);
}
</script>

<template>
  <div class="p-8 lg:p-10">
    <div class="flex items-center justify-between gap-4">
      <div>
        <p class="text-xs font-bold tracking-[0.2em] text-blue-500 uppercase dark:text-blue-200">Environment</p>
        <h2 class="mt-1 text-xl font-black text-slate-800 dark:text-slate-100">环境检查</h2>
      </div>
      <button
        type="button"
        class="inline-flex cursor-pointer items-center gap-1.5 rounded-lg px-3 py-2 text-sm font-bold text-blue-600 transition hover:bg-blue-50 disabled:cursor-not-allowed disabled:opacity-40 dark:text-blue-200 dark:hover:bg-blue-950"
        :disabled="busy"
        @click="recheckEnvironment"
      >
        <svg
          class="h-4 w-4"
          :class="{ 'animate-spin-twice': checking }"
          @animationend="checking = false"
          viewBox="0 0 24 24"
          fill="none"
          aria-hidden="true"
        >
          <path
            d="M20 6v5h-5M4 18v-5h5"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            d="M18.5 9A7 7 0 0 0 6.2 6.2L4 8m16 8-2.2 1.8A7 7 0 0 1 5.5 15"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
        重新检查
      </button>
    </div>

    <dl class="mt-7 space-y-1">
      <div
        v-for="row in statusRows"
        :key="row.label"
        class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-6 rounded-xl px-3 py-3 transition hover:bg-slate-50 dark:hover:bg-slate-800/60"
      >
        <dt class="flex items-center gap-3 font-semibold text-slate-600 dark:text-slate-400">
          <span class="h-2.5 w-2.5 shrink-0 rounded-full shadow-sm" :class="row.dotClass" aria-hidden="true"></span>
          {{ row.label }}
        </dt>
        <dd class="max-w-48 truncate text-base font-bold" :class="row.valueClass" :title="row.value">
          {{ row.value }}
        </dd>
      </div>
    </dl>
  </div>
</template>

<style scoped>
.animate-spin-twice {
  animation: spin-twice 1s linear 1;
}

@keyframes spin-twice {
  to {
    transform: rotate(720deg);
  }
}
</style>
