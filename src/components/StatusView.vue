<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import IconLink from "~icons/line-md/link";
import IconWeb from "~icons/streamline-plump/web";
import IconLog from "~icons/ix/log";
import IconDebugStart from "~icons/codicon/debug-start";
import logoUrl from "../assets/logo.png";
import type { RuntimeSnapshot } from "../composables/useRuntime";
import { checkEnv, checkVersion, installDsh, openLogDir, startServer } from "../composables/useRuntime";

const LINKS = {
  marketplace: "https://dshfind.com/zh",
  github: "https://github.com/festoney8/deepseek-harness-GUI",
  releases: "https://github.com/festoney8/deepseek-harness-GUI/releases",
  nodejs: "https://nodejs.org/zh-cn/download",
} as const;

const { state } = defineProps<{ state: RuntimeSnapshot }>();
/** 应用版本号，运行时读取（跟随 tauri.conf.json 的 version） */
const appVersion = ref("");
void getVersion().then((v) => {
  appVersion.value = v;
});

const LATEST_RELEASE_API = "https://api.github.com/repos/festoney8/deepseek-harness-GUI/releases/latest" as const;
/** GitHub 最新 release 版本号（失败时显示“未知”） */
const latestVersion = ref("检查中");
async function fetchLatestVersion() {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 5000);
  try {
    const res = await fetch(LATEST_RELEASE_API, { signal: controller.signal });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const data = (await res.json()) as { tag_name?: string };
    latestVersion.value = data.tag_name ?? "未知";
  } catch {
    latestVersion.value = "未知";
  } finally {
    clearTimeout(timer);
  }
}
void fetchLatestVersion();

/** 本工具 GitHub 最新版与当前版不一致（有新版本可升级），最新版号高亮；非版本号哨兵值（检查中/未知）不高亮 */
const versionOutdated = computed(() => {
  const latest = latestVersion.value;
  const current = appVersion.value;
  if (!/^v?\d/.test(latest) || !/^v?\d/.test(current)) return false;
  return latest.replace(/^v/, "") !== current.replace(/^v/, "");
});

const installing = computed(() => state.phase === "installing");
const starting = computed(() => state.phase === "starting");
const busy = computed(() => installing.value || starting.value);
const environmentReady = computed(() => Boolean(state.node && state.npm));
/** 远端与本地版本均已成功检测，安装/更新按钮才允许操作 */
const versionsReady = computed(() => state.versionChecked && !state.versionError);
/** 合并后的安装/更新按钮：环境就绪且本地与远端版本不同才可用 */
const installUpdateDisabled = computed(() => {
  if (busy.value || !environmentReady.value || !versionsReady.value) {
    return true;
  }
  return !(state.local === null || state.local !== state.remote);
});
/** 当前正在安装的源；只有被点击的按钮显示安装/更新中，另一个保持原文案 */
const installingSource = ref<"official" | "mirror" | null>(null);

watch(
  () => state.phase,
  (phase) => {
    if (phase !== "installing") installingSource.value = null;
  },
);

function startInstall(source: "official" | "mirror") {
  if (installUpdateDisabled.value) return;
  installingSource.value = source;
  installDsh(source === "mirror");
}

const installUpdateText = (source: "official" | "mirror", suffix: string) =>
  installing.value && installingSource.value === source
    ? state.local
      ? "更新中…"
      : "安装中…"
    : state.local === null
      ? `安装 DSH (${suffix})`
      : `更新 DSH (${suffix})`;

/** 安装/更新按钮文案：按需安装或更新（官方源） */
const installUpdateLabel = computed(() => installUpdateText("official", "官方源"));
/** 安装/更新按钮文案：按需安装或更新（镜像源） */
const installUpdateMirrorLabel = computed(() => installUpdateText("mirror", "镜像源"));
const startBlockReason = computed(() => {
  if (state.node == null || state.npm == null) return "请先安装 Node.js 环境";
  if (state.local == null) return "请先安装 DeepSeek Harness";
  return "";
});
const startDisabled = computed(() => busy.value || Boolean(startBlockReason.value));
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

function openExternal(url: string) {
  void openUrl(url);
}

/** 重新检查按钮 icon 的旋转动画进行中（转两圈后由 animationend 清除） */
const checking = ref(false);

function recheckEnvironment() {
  if (checking.value) return;
  checking.value = true;
  void Promise.allSettled([checkEnv(), checkVersion()]);
}
</script>

<template>
  <div
    class="h-full overflow-y-auto bg-[radial-gradient(circle_at_top,#eff6ff_0,#ffffff_42%,#f8fafc_100%)] px-8 py-8 text-slate-900 dark:bg-[radial-gradient(circle_at_top,#0f172a_0,#020617_42%,#0b1220_100%)] dark:text-slate-100"
  >
    <div class="mx-auto flex min-h-full w-full max-w-5xl flex-col justify-center gap-10">
      <header class="flex items-center justify-center gap-8">
        <div class="relative shrink-0">
          <div class="absolute inset-4 rounded-full bg-blue-400/20 blur-2xl" aria-hidden="true"></div>
          <img
            :src="logoUrl"
            alt="DeepSeek Harness GUI logo"
            class="relative h-36 w-36 object-contain drop-shadow-xl"
          />
        </div>

        <div class="min-w-0">
          <h1 class="text-4xl font-black text-[#315d9c] lg:text-5xl dark:text-blue-200">DeepSeek Harness GUI</h1>
          <nav class="mt-5 ml-1 flex items-center gap-4" aria-label="项目链接">
            <button
              type="button"
              class="group inline-flex cursor-pointer items-center gap-1.5 text-base font-bold text-[#315d9c] transition hover:text-blue-500 focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:ring-offset-4 focus-visible:outline-none dark:text-blue-200 dark:hover:text-blue-100"
              @click="openExternal(LINKS.marketplace)"
            >
              插件市场
              <IconLink class="h-4 w-4 transition group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
            </button>
            <span class="h-5 w-px bg-blue-200 dark:bg-blue-900" aria-hidden="true"></span>
            <button
              type="button"
              class="group inline-flex cursor-pointer items-center gap-1.5 text-base font-bold text-[#315d9c] transition hover:text-blue-500 focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:ring-offset-4 focus-visible:outline-none dark:text-blue-200 dark:hover:text-blue-100"
              @click="openExternal(LINKS.github)"
            >
              项目 GitHub
              <IconLink class="h-4 w-4 transition group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
            </button>
            <span class="h-5 w-px bg-blue-200 dark:bg-blue-900" aria-hidden="true"></span>
            <button
              type="button"
              class="group inline-flex cursor-pointer items-center gap-1.5 text-base font-bold transition focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:ring-offset-4 focus-visible:outline-none"
              :class="
                versionOutdated
                  ? 'text-amber-600 hover:text-amber-700 dark:text-amber-400 dark:hover:text-amber-300'
                  : 'text-[#315d9c] hover:text-blue-500 dark:text-blue-200 dark:hover:text-blue-100'
              "
              @click="openExternal(LINKS.releases)"
            >
              最新 {{ latestVersion }}
              <IconLink class="h-4 w-4 transition group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
            </button>
            <span class="h-5 w-px bg-blue-200 dark:bg-blue-900" aria-hidden="true"></span>
            <span class="text-base font-bold text-[#315d9c] dark:text-blue-200">当前 v{{ appVersion }}</span>
          </nav>
        </div>
      </header>

      <section
        class="grid overflow-hidden rounded-3xl border border-blue-200 bg-white/95 shadow-[0_24px_70px_-35px_rgba(37,99,235,0.5)] md:grid-cols-[1.05fr_0.95fr] dark:border-blue-900 dark:bg-slate-900/95 dark:shadow-[0_24px_70px_-35px_rgba(2,6,23,0.9)]"
      >
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
                <span
                  class="h-2.5 w-2.5 shrink-0 rounded-full shadow-sm"
                  :class="row.dotClass"
                  aria-hidden="true"
                ></span>
                {{ row.label }}
              </dt>
              <dd class="max-w-48 truncate text-base font-bold" :class="row.valueClass" :title="row.value">
                {{ row.value }}
              </dd>
            </div>
          </dl>
        </div>

        <div
          class="relative border-t border-dashed border-blue-300 bg-blue-50/35 p-8 md:border-t-0 md:border-l lg:p-10 dark:border-blue-800 dark:bg-blue-950/40"
        >
          <span
            class="absolute top-8 -left-1 hidden h-2 w-2 rounded-full bg-blue-400 md:block"
            aria-hidden="true"
          ></span>
          <span
            class="absolute bottom-8 -left-1 hidden h-2 w-2 rounded-full bg-blue-400 md:block"
            aria-hidden="true"
          ></span>

          <p class="text-xs font-bold tracking-[0.2em] text-blue-500 uppercase">Actions</p>
          <h2 class="mt-1 text-xl font-black text-slate-800 dark:text-slate-100">快捷操作</h2>

          <div class="mt-9 grid grid-cols-2 gap-3">
            <button
              type="button"
              class="col-start-1 row-start-1 inline-flex min-h-12 cursor-pointer items-center justify-center gap-2 rounded-xl border border-blue-200 bg-white px-4 font-bold text-blue-700 shadow-sm transition hover:-translate-y-0.5 hover:border-blue-300 hover:shadow-md focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:outline-none disabled:cursor-not-allowed disabled:border-slate-300/60 disabled:bg-slate-200/60 disabled:text-slate-500/70 disabled:shadow-none disabled:hover:translate-y-0 disabled:hover:border-slate-300/60 disabled:hover:bg-slate-200/60 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100 dark:hover:bg-slate-700 dark:disabled:border-slate-700/60 dark:disabled:bg-slate-800/60 dark:disabled:text-slate-500/70 dark:disabled:hover:border-slate-700/60 dark:disabled:hover:bg-slate-800/60"
              @click="openExternal(LINKS.nodejs)"
            >
              <IconWeb class="h-4.5 w-4.5" />
              Node.js 官网
            </button>

            <button
              type="button"
              class="col-start-1 row-start-2 inline-flex min-h-12 cursor-pointer items-center justify-center rounded-xl border border-blue-200 bg-white px-4 font-bold text-blue-700 shadow-sm transition hover:-translate-y-0.5 hover:border-blue-300 hover:shadow-md focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:outline-none disabled:cursor-not-allowed disabled:border-slate-300/60 disabled:bg-slate-200/60 disabled:text-slate-500/70 disabled:shadow-none disabled:hover:translate-y-0 disabled:hover:border-slate-300/60 disabled:hover:bg-slate-200/60 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100 dark:hover:bg-slate-700 dark:disabled:border-slate-700/60 dark:disabled:bg-slate-800/60 dark:disabled:text-slate-500/70 dark:disabled:hover:border-slate-700/60 dark:disabled:hover:bg-slate-800/60"
              :disabled="installUpdateDisabled"
              @click="startInstall('official')"
            >
              {{ installUpdateLabel }}
            </button>
            <button
              type="button"
              class="col-start-2 row-start-2 inline-flex min-h-12 cursor-pointer items-center justify-center rounded-xl border border-blue-200 bg-white px-4 font-bold text-blue-700 shadow-sm transition hover:-translate-y-0.5 hover:border-blue-300 hover:shadow-md focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:outline-none disabled:cursor-not-allowed disabled:border-slate-300/60 disabled:bg-slate-200/60 disabled:text-slate-500/70 disabled:shadow-none disabled:hover:translate-y-0 disabled:hover:border-slate-300/60 disabled:hover:bg-slate-200/60 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100 dark:hover:bg-slate-700 dark:disabled:border-slate-700/60 dark:disabled:bg-slate-800/60 dark:disabled:text-slate-500/70 dark:disabled:hover:border-slate-700/60 dark:disabled:hover:bg-slate-800/60"
              :disabled="installUpdateDisabled"
              @click="startInstall('mirror')"
            >
              {{ installUpdateMirrorLabel }}
            </button>

            <button
              type="button"
              class="col-span-2 row-start-3 mt-3 inline-flex min-h-14 cursor-pointer items-center justify-center gap-3 rounded-xl bg-blue-600 px-5 text-lg font-black text-white shadow-lg shadow-blue-200 transition hover:-translate-y-0.5 hover:bg-blue-700 hover:shadow-xl hover:shadow-blue-200 focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:bg-slate-300/60 disabled:text-slate-500/70 disabled:shadow-none disabled:hover:translate-y-0 disabled:hover:bg-slate-300/60 dark:shadow-blue-950 dark:hover:shadow-blue-950 dark:disabled:bg-slate-700/60 dark:disabled:text-slate-400/70 dark:disabled:hover:bg-slate-700/60"
              :disabled="startDisabled"
              :aria-busy="starting"
              @click="startServer"
            >
              <IconDebugStart v-if="!starting" class="h-5 w-5" />
              <svg v-else class="h-5 w-5 animate-spin" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <circle class="opacity-30" cx="12" cy="12" r="9" stroke="currentColor" stroke-width="3" />
                <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="3" stroke-linecap="round" />
              </svg>
              {{ starting ? "正在运行…" : "运行 DeepSeek Harness" }}
            </button>
            <p
              v-if="startBlockReason"
              class="col-span-2 row-start-4 text-center text-xs font-medium text-rose-500 dark:text-rose-400"
            >
              {{ startBlockReason }}
            </p>

            <button
              type="button"
              class="col-start-2 row-start-1 inline-flex min-h-12 cursor-pointer items-center justify-center gap-2 rounded-xl border border-blue-200 bg-white px-4 font-bold text-blue-700 shadow-sm transition hover:-translate-y-0.5 hover:border-blue-300 hover:shadow-md focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:outline-none disabled:cursor-not-allowed disabled:border-slate-300/60 disabled:bg-slate-200/60 disabled:text-slate-500/70 disabled:shadow-none disabled:hover:translate-y-0 disabled:hover:border-slate-300/60 disabled:hover:bg-slate-200/60 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100 dark:hover:bg-slate-700 dark:disabled:border-slate-700/60 dark:disabled:bg-slate-800/60 dark:disabled:text-slate-500/70 dark:disabled:hover:border-slate-700/60 dark:disabled:hover:bg-slate-800/60"
              @click="openLogDir"
            >
              <IconLog class="h-4.5 w-4.5" />
              查看日志
            </button>
          </div>
        </div>
      </section>
    </div>
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
