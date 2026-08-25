<template>
  <dialog ref="dialogEl" class="modal">
    <div class="modal-box relative w-11/12 max-w-3xl">
      <h3 class="mb-4 text-2xl font-bold">插件市场合集</h3>
      <form method="dialog" class="absolute right-2 top-2">
        <button class="btn btn-circle btn-ghost" type="submit" aria-label="关闭">
          <CloseIcon class="size-6" aria-hidden="true" />
        </button>
      </form>

      <div v-if="loadState.kind === 'checking'" class="flex justify-center py-12">
        <span class="loading loading-spinner loading-lg" aria-label="加载中"></span>
      </div>

      <div v-else-if="loadState.kind === 'error'" class="flex flex-col items-center gap-3 py-8">
        <p class="text-base text-error">加载失败：{{ loadState.message }}</p>
        <button class="btn btn-sm" type="button" @click="loadPlugins">重试</button>
      </div>

      <p v-else-if="plugins.length === 0" class="py-8 text-center text-base-content/70">暂无插件</p>

      <div v-else class="grid grid-cols-2 gap-3">
        <div
          v-for="plugin in plugins"
          :key="plugin.url"
          class="flex items-center gap-3 rounded-box border border-base-300 p-3"
        >
          <PluginIcon class="plugin-icon size-10 shrink-0" aria-hidden="true" />
          <div class="min-w-0 flex-1">
            <div class="truncate text-lg font-bold" :title="plugin.title">{{ plugin.title }}</div>
            <div class="truncate text-sm text-base-content/60" :title="plugin.url">{{ plugin.url }}</div>
          </div>
          <button class="btn btn-sm btn-outline btn-primary shrink-0" type="button" @click="visit(plugin)">访问</button>
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button>close</button></form>
    <ToastViewport />
  </dialog>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import CloseIcon from "~icons/material-symbols/close";
import PluginIcon from "~icons/streamline-freehand/plugin-jigsaw-puzzle";
import { fetchFirstValidJson } from "@/utils/http";
import { getErrorMessage, useToast } from "@/composables/useToast";
import ToastViewport from "../feedback/ToastViewport.vue";

/**
 * 插件市场数据地址
 * https://raw.githubusercontent.com/festoney8/deepseek-harness-GUI/refs/heads/data/plugins/market.json
 */
const MARKET_JSON_URLS = [
  "https://raw.githubusercontent.com/festoney8/deepseek-harness-GUI/refs/heads/data/plugins/market.json",
  "https://axisnow.gh-proxy.org/https://raw.githubusercontent.com/festoney8/deepseek-harness-GUI/refs/heads/data/plugins/market.json",
] as const;

/** 市场条目 */
interface MarketPlugin {
  title: string;
  url: string;
}

/**
 * 加载状态：尚未打开 / 获取中 / 成功 / 失败
 * 与 env.ts 的 VersionState 同一判别联合风格
 */
type MarketLoadState = { kind: "idle" } | { kind: "checking" } | { kind: "ok" } | { kind: "error"; message: string };

const dialogEl = ref<HTMLDialogElement | null>(null);
const plugins = ref<MarketPlugin[]>([]);
const loadState = ref<MarketLoadState>({ kind: "idle" });

const toast = useToast();

/** 校验响应并提取合法条目；结构不符时抛错走 error 态 */
function parsePlugins(data: unknown): MarketPlugin[] {
  const list = (data as { plugins?: unknown } | null)?.plugins;
  if (!Array.isArray(list)) {
    throw new Error("响应缺少 plugins 数组");
  }
  return list.filter((item): item is MarketPlugin => {
    const plugin = item as Record<string, unknown>;
    return (
      typeof plugin?.title === "string" && plugin.title !== "" && typeof plugin?.url === "string" && plugin.url !== ""
    );
  });
}

/** 拉取市场数据并驱动 loading / ok / error 状态切换 */
async function loadPlugins(): Promise<void> {
  loadState.value = { kind: "checking" };
  try {
    plugins.value = await fetchFirstValidJson(MARKET_JSON_URLS, parsePlugins);
    loadState.value = { kind: "ok" };
  } catch (cause) {
    console.warn("pluginMarket", "加载插件市场数据失败:", cause);
    loadState.value = { kind: "error", message: getErrorMessage(cause) };
  }
}

/** 打开 modal 后才开始获取，每次打开都重新拉取保持数据新鲜 */
function open(): void {
  dialogEl.value?.showModal();
  void loadPlugins();
}

/** 用系统默认浏览器打开插件页面 */
async function visit(plugin: MarketPlugin): Promise<void> {
  try {
    await openUrl(plugin.url);
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}

defineExpose({ open });
</script>

<style lang="scss" scoped>
[data-theme="night"] {
  .plugin-icon {
    color: rgb(200, 209, 255);
  }
}
</style>
