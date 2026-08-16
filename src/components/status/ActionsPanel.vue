<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useLocalStorage } from "@vueuse/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import IconWeb from "~icons/streamline-plump/web";
import IconLog from "~icons/ix/log";
import IconDebugStart from "~icons/codicon/debug-start";
import type { RuntimeSnapshot } from "../../composables/useRuntime";
import { installDsh, openLogDir, startServer } from "../../composables/useRuntime";

const LINKS = {
  nodejs: "https://nodejs.org/zh-cn/download",
} as const;

const { state } = defineProps<{ state: RuntimeSnapshot }>();

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
/** 上次运行时使用的连接地址（点击运行时写入，下次启动恢复） */
const storedHost = useLocalStorage<string>("dsh-connect-host", "127.0.0.1");
const storedPort = useLocalStorage<number>("dsh-connect-port", 3080);
/** 输入框当前值：初始化自 localStorage，仅在点击运行时回写 */
const hostInput = ref(storedHost.value);
const portInput = ref(String(storedPort.value));

/** 主机合法性：localhost 或合法 IPv4（非法时高亮输入框并禁用运行按钮；IPv6/域名一律非法） */
const hostValid = computed(() => {
  const h = hostInput.value.trim().toLowerCase();
  if (h === "localhost") return true;
  const parts = h.split(".");
  return parts.length === 4 && parts.every((p) => /^\d{1,3}$/.test(p) && Number(p) <= 255);
});

/** 端口合法性：1~65535 整数（非法时高亮输入框并禁用运行按钮） */
const portValid = computed(() => {
  const n = Number(portInput.value.trim());
  return Number.isInteger(n) && n >= 1 && n <= 65535;
});

/** 主机是否为本地地址；决定按钮文案"运行/连接" */
const isLocalHost = computed(() => {
  const h = hostInput.value.trim().toLowerCase();
  return h === "localhost" || h === "127.0.0.1";
});

/** 运行按钮文案：本地地址显示"运行"，远程地址显示"连接" */
const startLabel = computed(() =>
  starting.value
    ? isLocalHost.value
      ? "正在运行…"
      : "正在连接…"
    : isLocalHost.value
      ? "运行 DeepSeek Harness"
      : "连接 DeepSeek Harness",
);

const startBlockReason = computed(() => {
  if (!hostValid.value) return "主机需为 localhost 或合法 IPv4 地址";
  if (!portValid.value) return "端口需为 1~65535 的整数";
  if (state.node == null || state.npm == null) return "请先安装 Node.js 环境";
  if (state.local == null) return "请先安装 DeepSeek Harness";
  return "";
});
const startDisabled = computed(() => busy.value || Boolean(startBlockReason.value));

/** 记录本次使用的连接值并启动：host 为本地地址时启动 dsh，否则连接远程服务 */
function runServer() {
  if (startDisabled.value) return;
  const host = hostInput.value.trim();
  const portNum = Number(portInput.value.trim());
  storedHost.value = host;
  storedPort.value = portNum;
  startServer(host, portNum);
}

function openExternal(url: string) {
  void openUrl(url);
}
</script>

<template>
  <div
    class="relative border-t border-dashed border-blue-300 bg-blue-50/35 p-8 md:border-t-0 md:border-l lg:p-10 dark:border-blue-800 dark:bg-blue-950/40"
  >
    <span class="absolute top-8 -left-1 hidden h-2 w-2 rounded-full bg-blue-400 md:block" aria-hidden="true"></span>
    <span class="absolute bottom-8 -left-1 hidden h-2 w-2 rounded-full bg-blue-400 md:block" aria-hidden="true"></span>

    <p class="text-xs font-bold tracking-[0.2em] text-blue-500 uppercase">Actions</p>
    <h2 class="mt-1 text-xl font-black text-slate-800 dark:text-slate-100">快捷操作</h2>

    <div class="mt-9 grid grid-cols-2 gap-3">
      <button
        type="button"
        class="col-start-1 row-start-1 inline-flex min-h-12 cursor-pointer items-center justify-center gap-2 rounded-xl border border-blue-200 bg-white px-4 font-bold text-blue-700 shadow-sm transition hover:-translate-y-0.5 hover:border-blue-300 hover:shadow-md focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:outline-none disabled:cursor-not-allowed disabled:border-slate-300/60 disabled:bg-slate-200/60 disabled:text-slate-500/70 disabled:shadow-none disabled:hover:translate-y-0 disabled:hover:border-slate-300/60 disabled:hover:bg-slate-200/60 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100 dark:hover:bg-slate-700 dark:disabled:border-slate-700/60 dark:disabled:bg-slate-800/60 dark:disabled:text-slate-500/70 dark:disabled:hover:translate-y-0 dark:disabled:hover:border-slate-700/60 dark:disabled:hover:bg-slate-800/60"
        @click="openExternal(LINKS.nodejs)"
      >
        <IconWeb class="h-4.5 w-4.5" />
        Node.js 官网
      </button>

      <button
        type="button"
        class="col-start-1 row-start-2 inline-flex min-h-12 cursor-pointer items-center justify-center rounded-xl border border-blue-200 bg-white px-4 font-bold text-blue-700 shadow-sm transition hover:-translate-y-0.5 hover:border-blue-300 hover:shadow-md focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:outline-none disabled:cursor-not-allowed disabled:border-slate-300/60 disabled:bg-slate-200/60 disabled:text-slate-500/70 disabled:shadow-none disabled:hover:translate-y-0 disabled:hover:border-slate-300/60 disabled:hover:bg-slate-200/60 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100 dark:hover:bg-slate-700 dark:disabled:border-slate-700/60 dark:disabled:bg-slate-800/60 dark:disabled:text-slate-500/70 dark:disabled:hover:translate-y-0 dark:disabled:hover:border-slate-700/60 dark:disabled:hover:bg-slate-800/60"
        :disabled="installUpdateDisabled"
        @click="startInstall('official')"
      >
        {{ installUpdateLabel }}
      </button>
      <button
        type="button"
        class="col-start-2 row-start-2 inline-flex min-h-12 cursor-pointer items-center justify-center rounded-xl border border-blue-200 bg-white px-4 font-bold text-blue-700 shadow-sm transition hover:-translate-y-0.5 hover:border-blue-300 hover:shadow-md focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:outline-none disabled:cursor-not-allowed disabled:border-slate-300/60 disabled:bg-slate-200/60 disabled:text-slate-500/70 disabled:shadow-none disabled:hover:translate-y-0 disabled:hover:border-slate-300/60 disabled:hover:bg-slate-200/60 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100 dark:hover:bg-slate-700 dark:disabled:border-slate-700/60 dark:disabled:bg-slate-800/60 dark:disabled:text-slate-500/70 dark:disabled:hover:translate-y-0 dark:disabled:hover:border-slate-700/60 dark:disabled:hover:bg-slate-800/60"
        :disabled="installUpdateDisabled"
        @click="startInstall('mirror')"
      >
        {{ installUpdateMirrorLabel }}
      </button>

      <div class="col-span-2 row-start-3 flex items-center gap-2">
        <label for="connect-host" class="mr-2 shrink-0 text-base font-bold text-slate-500 dark:text-slate-400">
          主机
        </label>
        <input
          id="connect-host"
          v-model="hostInput"
          type="text"
          spellcheck="false"
          placeholder="127.0.0.1"
          :class="[
            'min-w-0 flex-1 rounded-xl border bg-white px-4 py-2.5 text-base font-semibold text-slate-800 shadow-sm transition focus:outline-none dark:bg-slate-800 dark:text-slate-100',
            hostValid
              ? 'border-blue-200 focus:border-blue-400 focus:ring-2 focus:ring-blue-200 dark:border-slate-600 dark:focus:border-blue-500 dark:focus:ring-blue-900'
              : 'border-rose-400 focus:border-rose-500 focus:ring-2 focus:ring-rose-200 dark:border-rose-500/70 dark:focus:ring-rose-900/50',
          ]"
        />
        <label for="connect-port" class="mr-2 ml-3 shrink-0 text-base font-bold text-slate-500 dark:text-slate-400">
          端口
        </label>
        <input
          id="connect-port"
          v-model="portInput"
          type="text"
          inputmode="numeric"
          maxlength="5"
          placeholder="3080"
          :class="[
            'w-28 rounded-xl border bg-white px-3 py-2.5 text-center text-base font-bold text-slate-800 shadow-sm transition focus:outline-none dark:bg-slate-800 dark:text-slate-100',
            portValid
              ? 'border-blue-200 focus:border-blue-400 focus:ring-2 focus:ring-blue-200 dark:border-slate-600 dark:focus:border-blue-500 dark:focus:ring-blue-900'
              : 'border-rose-400 focus:border-rose-500 focus:ring-2 focus:ring-rose-200 dark:border-rose-500/70 dark:focus:ring-rose-900/50',
          ]"
        />
      </div>

      <button
        type="button"
        class="col-span-2 row-start-4 mt-3 inline-flex min-h-14 cursor-pointer items-center justify-center gap-3 rounded-xl bg-blue-600 px-5 text-lg font-black text-white shadow-lg shadow-blue-200 transition hover:-translate-y-0.5 hover:bg-blue-700 hover:shadow-xl hover:shadow-blue-200 focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:bg-slate-300/60 disabled:text-slate-500/70 disabled:shadow-none disabled:hover:translate-y-0 disabled:hover:bg-slate-300/60 dark:shadow-blue-950 dark:hover:shadow-blue-950 dark:disabled:bg-slate-700/60 dark:disabled:text-slate-400/70 dark:disabled:hover:bg-slate-700/60"
        :disabled="startDisabled"
        :aria-busy="starting"
        @click="runServer"
      >
        <IconDebugStart v-if="!starting" class="h-5 w-5" />
        <svg v-else class="h-5 w-5 animate-spin" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <circle class="opacity-30" cx="12" cy="12" r="9" stroke="currentColor" stroke-width="3" />
          <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="3" stroke-linecap="round" />
        </svg>
        {{ startLabel }}
      </button>
      <p
        v-if="startBlockReason"
        class="col-span-2 row-start-5 text-center text-xs font-medium text-rose-500 dark:text-rose-400"
      >
        {{ startBlockReason }}
      </p>
      <p
        v-if="state.phase === 'failed' && state.detail"
        class="col-span-2 row-start-6 text-center text-xs font-medium text-rose-600 dark:text-rose-400"
      >
        {{ state.detail }}
      </p>

      <button
        type="button"
        class="col-start-2 row-start-1 inline-flex min-h-12 cursor-pointer items-center justify-center gap-2 rounded-xl border border-blue-200 bg-white px-4 font-bold text-blue-700 shadow-sm transition hover:-translate-y-0.5 hover:border-blue-300 hover:shadow-md focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:outline-none disabled:cursor-not-allowed disabled:border-slate-300/60 disabled:bg-slate-200/60 disabled:text-slate-500/70 disabled:shadow-none disabled:hover:translate-y-0 disabled:hover:border-slate-300/60 disabled:hover:bg-slate-200/60 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100 dark:hover:bg-slate-700 dark:disabled:border-slate-700/60 dark:disabled:bg-slate-800/60 dark:disabled:text-slate-500/70 dark:disabled:hover:translate-y-0 dark:disabled:hover:border-slate-700/60 dark:disabled:hover:bg-slate-800/60"
        @click="openLogDir"
      >
        <IconLog class="h-4.5 w-4.5" />
        查看日志
      </button>
    </div>
  </div>
</template>
