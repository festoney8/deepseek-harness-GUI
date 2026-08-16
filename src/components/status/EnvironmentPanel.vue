<script setup lang="ts">
import { computed, ref } from "vue";
import type { RuntimeSnapshot } from "../../composables/useRuntime";
import { busy, checkEnv, checkVersion } from "../../composables/useRuntime";

interface StatusRow {
  label: string;
  value: string;
  valueClass: string;
  status: "info" | "warning" | "error";
  loading: boolean;
}

const { state } = defineProps<{ state: RuntimeSnapshot }>();

const versionRow = (label: string, value: string | null, failed: boolean, highlighted = false): StatusRow => {
  const loading = value === null && !failed;
  return {
    label,
    value: value ?? (failed ? "获取失败" : "正在加载"),
    valueClass: value ? (highlighted ? "text-warning" : "text-base-content") : failed ? "text-error" : "text-warning",
    status: failed ? "error" : "info",
    loading,
  };
};

const environmentRow = (
  label: string,
  value: string | null,
  missing: { text?: string; tone?: "warning" | "error" } = {},
): StatusRow => {
  const loading = value === null && !state.versionChecked;
  const missingTone = missing.tone ?? "error";
  return {
    label,
    value: value ?? (loading ? "正在加载" : (missing.text ?? "未检测到")),
    valueClass: value ? "text-base-content" : loading || missingTone === "warning" ? "text-warning" : "text-error",
    status: value ? "info" : loading ? "warning" : missingTone,
    loading,
  };
};

const statusRows = computed<StatusRow[]>(() => [
  environmentRow("node 版本（推荐 v24 及以上）", state.node),
  environmentRow("npm 版本", state.npm),
  versionRow(
    "最新 DSH 版本（官方源）",
    state.remote,
    state.versionError,
    state.local !== null && state.remote !== state.local,
  ),
  versionRow(
    "最新 DSH 版本（镜像源）",
    state.remoteMirror,
    state.versionChecked && state.remoteMirror === null,
    state.local !== null && state.remoteMirror !== state.local,
  ),
  environmentRow("本地 DSH 版本", state.local, { text: "未安装", tone: "warning" }),
]);

const checking = ref(false);

function recheckEnvironment() {
  if (checking.value) return;
  checking.value = true;
  void Promise.allSettled([checkEnv(), checkVersion()]).finally(() => {
    window.setTimeout(() => {
      checking.value = false;
    }, 1200);
  });
}
</script>

<template>
  <section class="min-w-0 p-8 lg:p-10" aria-labelledby="environment-title">
    <div class="flex items-start justify-between gap-4">
      <div class="pl-3">
        <h2 id="environment-title" class="text-base-content text-xl font-black">环境检查</h2>
      </div>
      <button
        type="button"
        class="btn btn-ghost btn-sm text-primary shrink-0 gap-1.5 rounded-xl text-base"
        :disabled="checking || busy"
        @click="recheckEnvironment"
      >
        <span v-if="checking" class="loading loading-spinner loading-xs" aria-label="正在检查"></span>
        <svg v-else class="h-4 w-4" viewBox="0 0 24 24" fill="none" aria-hidden="true">
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
        {{ checking ? "正在检查" : "重新检查" }}
      </button>
    </div>

    <ul class="list mt-6 w-full" aria-label="环境和版本状态">
      <li
        v-for="row in statusRows"
        :key="row.label"
        class="list-row grid grid-cols-[minmax(0,1fr)_auto] items-center gap-4 rounded-xl px-3 py-3"
      >
        <div class="text-base-content/70 flex min-w-0 items-center gap-3 font-semibold">
          <span
            class="status status-md shrink-0"
            :class="[row.loading ? 'status-warning animate-bounce' : `status-${row.status}`]"
            :aria-label="row.loading ? '正在加载' : undefined"
            :aria-hidden="!row.loading"
          ></span>
          <span class="truncate" :title="row.label">{{ row.label }}</span>
        </div>
        <span class="max-w-56 truncate text-right text-base font-bold" :class="row.valueClass" :title="row.value">
          {{ row.value }}
        </span>
      </li>
    </ul>
  </section>
</template>
