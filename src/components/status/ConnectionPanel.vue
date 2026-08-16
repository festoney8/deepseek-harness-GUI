<script setup lang="ts">
import { computed, ref } from "vue";
import { useLocalStorage } from "@vueuse/core";
import IconDebugStart from "~icons/codicon/debug-start";
import IconLink from "~icons/line-md/link";
import type { RuntimeSnapshot } from "../../composables/useRuntime";
import { startServer } from "../../composables/useRuntime";
import { beginAction, reportActionError } from "../../composables/useActionFeedback";

const { state } = defineProps<{ state: RuntimeSnapshot }>();
const storedHost = useLocalStorage("dsh-connect-host", "127.0.0.1");
const storedPort = useLocalStorage("dsh-connect-port", 3080);
const hostInput = ref(storedHost.value);
const portInput = ref(String(storedPort.value));

const starting = computed(() => state.phase === "starting");
const busy = computed(() => state.phase === "installing" || starting.value);
const hostValid = computed(() => {
  const host = hostInput.value.trim().toLowerCase();
  if (host === "localhost") return true;
  const parts = host.split(".");
  return parts.length === 4 && parts.every((part) => /^\d{1,3}$/.test(part) && Number(part) <= 255);
});
const portValid = computed(() => {
  const port = Number(portInput.value.trim());
  return Number.isInteger(port) && port >= 1 && port <= 65535;
});
const isLocalHost = computed(() => {
  const host = hostInput.value.trim().toLowerCase();
  return host === "localhost" || host === "127.0.0.1";
});
const prerequisitesReady = computed(() => state.node !== null && state.npm !== null && state.local !== null);
const startAlert = computed(() => {
  if (!isLocalHost.value) return null;
  if (!state.versionChecked) {
    return { tone: "info", text: "正在加载环境信息…", loading: true } as const;
  }
  if (state.node === null || state.npm === null) {
    return { tone: "warning", text: "请先安装 Node.js 环境", loading: false } as const;
  }
  if (state.local === null) {
    return { tone: "warning", text: "请先安装 DeepSeek Harness", loading: false } as const;
  }
  return null;
});
const startDisabled = computed(
  () => busy.value || !hostValid.value || !portValid.value || (isLocalHost.value && !prerequisitesReady.value),
);
const startLabel = computed(() => {
  if (starting.value) return isLocalHost.value ? "正在运行…" : "正在连接…";
  return isLocalHost.value ? "运行 DSH" : "连接 DSH";
});

function runServer() {
  if (startDisabled.value) return;
  const host = hostInput.value.trim();
  const port = Number(portInput.value.trim());
  storedHost.value = host;
  storedPort.value = port;
  beginAction();
  void startServer(host, port).catch(reportActionError);
}
</script>

<template>
  <section class="min-w-0 p-8 lg:p-10" aria-labelledby="connection-title">
    <h2 id="connection-title" class="text-base-content text-xl font-black">本地运行 / 远程连接</h2>

    <div class="mt-6 space-y-3">
      <div>
        <label
          class="input dsh-input-shell focus-within:border-primary focus-within:ring-primary/20 w-full rounded-xl focus-within:ring-2 focus-within:outline-none"
          :class="{ 'input-error focus-within:border-error focus-within:ring-error/20': !hostValid }"
        >
          <span class="label text-base">主机</span>
          <input
            v-model="hostInput"
            class="validator min-w-0 focus:outline-none"
            type="text"
            required
            pattern="localhost|([0-9]{1,3}\.){3}[0-9]{1,3}"
            spellcheck="false"
            placeholder="127.0.0.1"
            :aria-invalid="!hostValid"
          />
        </label>
      </div>

      <div>
        <label
          class="input dsh-input-shell focus-within:border-primary focus-within:ring-primary/20 w-full rounded-xl focus-within:ring-2 focus-within:outline-none"
          :class="{ 'input-error focus-within:border-error focus-within:ring-error/20': !portValid }"
        >
          <span class="label text-base">端口</span>
          <input
            v-model="portInput"
            class="validator tabular-nums focus:outline-none"
            type="text"
            required
            pattern="[0-9]+"
            inputmode="numeric"
            maxlength="5"
            placeholder="3080"
            :aria-invalid="!portValid"
          />
        </label>
      </div>
    </div>

    <button
      type="button"
      class="btn btn-wide mt-5 min-h-14 w-full rounded-xl text-lg font-black disabled:opacity-100"
      :class="startDisabled ? 'btn-outline btn-neutral' : 'btn-primary'"
      :disabled="startDisabled"
      :aria-busy="starting"
      @click="runServer"
    >
      <span v-if="starting" class="loading loading-spinner loading-sm" :aria-label="startLabel"></span>
      <IconDebugStart v-else-if="isLocalHost" class="h-5 w-5" />
      <IconLink v-else class="h-5 w-5" />
      {{ startLabel }}
    </button>

    <div
      v-if="startAlert"
      role="alert"
      class="alert alert-outline mt-3 items-start rounded-xl px-3 py-2 text-sm"
      :class="startAlert.tone === 'warning' ? 'alert-warning' : 'alert-info'"
    >
      <span v-if="startAlert.loading" class="loading loading-spinner loading-xs mt-0.5 shrink-0"></span>
      <span>{{ startAlert.text }}</span>
    </div>
  </section>
</template>
