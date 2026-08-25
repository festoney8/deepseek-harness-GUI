<template>
  <dialog ref="dialogEl" class="modal">
    <div class="modal-box relative w-11/12 max-w-5xl">
      <h3 class="mb-6 text-2xl font-bold">推荐插件（安装后需重启 DSH）</h3>
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

      <p v-else-if="plugins.length === 0" class="py-8 text-center text-base-content/70">暂无推荐插件</p>

      <div v-else class="max-h-[calc(100vh-10rem)] overflow-y-auto pr-1">
        <div class="grid grid-cols-2 gap-3">
          <article
            v-for="plugin in plugins"
            :key="plugin.package"
            class="grid grid-cols-[3rem_minmax(0,1fr)_6rem] items-center gap-4 rounded-box border border-base-300 p-4"
          >
            <PluginIcon class="plugin-icon size-12 shrink-0" aria-hidden="true" />
            <div class="min-w-0">
              <div class="truncate text-lg font-bold" :title="plugin.title">{{ plugin.title }}</div>
              <p class="line-clamp-3 text-sm text-base-content/80" :title="plugin.description">
                {{ plugin.description }}
              </p>
            </div>
            <div class="flex flex-col items-end gap-3">
              <button
                class="btn btn-outline btn-sm"
                type="button"
                :aria-label="`打开 ${plugin.title} 的 GitHub 仓库`"
                @click="visitGithub(plugin)"
              >
                <GithubIcon class="size-5" aria-hidden="true" />
                <span>仓库</span>
              </button>
              <button
                class="btn btn-outline btn-sm btn-primary"
                type="button"
                :disabled="isInstalling(plugin.package) || isInstalled(plugin)"
                @click="installPlugin(plugin)"
              >
                <InstallIcon class="size-5" aria-hidden="true" />
                <span>{{ actionLabel(plugin) }}</span>
              </button>
            </div>
          </article>
        </div>
      </div>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import CloseIcon from "~icons/material-symbols/close";
import GithubIcon from "~icons/mdi/github";
import InstallIcon from "~icons/clarity/install-line";
import PluginIcon from "~icons/catppuccin/folder-plugins-open";
import { useDshPluginList, type DshPlugin } from "../../composables/useDshPluginList";
import { useInstallDshPlugin } from "../../composables/useInstallDshPlugin";
import { getErrorMessage, useToast } from "../../composables/useToast";
import { fetchJson } from "../../utils/http";
import { logger } from "../../utils/log";

/**
 * 推荐插件数据地址
 * https://raw.githubusercontent.com/festoney8/deepseek-harness-GUI/refs/heads/data/plugins/recommend.json
 */
const RECOMMEND_JSON_URL =
  "https://axisnow.gh-proxy.org/https://raw.githubusercontent.com/festoney8/deepseek-harness-GUI/refs/heads/data/plugins/recommend.json";

interface RecommendedPlugin {
  package: string;
  title: string;
  description: string;
  command: string;
  github: string;
}

type LoadState = { kind: "idle" } | { kind: "checking" } | { kind: "ok" } | { kind: "error"; message: string };
type InstallMode = "install" | "update";

const dialogEl = ref<HTMLDialogElement | null>(null);
const plugins = ref<RecommendedPlugin[]>([]);
const installedPlugins = ref<DshPlugin[]>([]);
const installingPackages = ref<string[]>([]);
const installingModes = ref<Record<string, InstallMode>>({});
const loadState = ref<LoadState>({ kind: "idle" });

const toast = useToast();

function parsePlugins(data: unknown): RecommendedPlugin[] {
  const list = (data as { plugins?: unknown } | null)?.plugins;
  if (!Array.isArray(list)) {
    throw new Error("响应缺少 plugins 数组");
  }
  return list.filter((item): item is RecommendedPlugin => {
    const plugin = item as Record<string, unknown>;
    return ["package", "title", "description", "command", "github"].every(
      (field) => typeof plugin[field] === "string" && plugin[field] !== "",
    );
  });
}

async function loadPlugins(): Promise<void> {
  loadState.value = { kind: "checking" };
  try {
    const [recommendData, localPlugins] = await Promise.all([fetchJson(RECOMMEND_JSON_URL), useDshPluginList()]);
    plugins.value = parsePlugins(recommendData);
    installedPlugins.value = localPlugins;
    loadState.value = { kind: "ok" };
  } catch (cause) {
    logger.warn("recommendedPlugins", "加载推荐插件失败:", cause);
    loadState.value = { kind: "error", message: getErrorMessage(cause) };
  }
}

function isInstalled(plugin: RecommendedPlugin): boolean {
  return installedPlugins.value.some((installed) => installed.name === plugin.package);
}

function isInstalling(packageName: string): boolean {
  return installingPackages.value.includes(packageName);
}

function actionLabel(plugin: RecommendedPlugin): string {
  const installingMode = installingModes.value[plugin.package];
  if (installingMode === "install") return "安装中";
  if (installingMode === "update") return "更新中";
  return "安装";
}

async function visitGithub(plugin: RecommendedPlugin): Promise<void> {
  try {
    await openUrl(plugin.github);
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}

async function installPlugin(plugin: RecommendedPlugin): Promise<void> {
  if (isInstalling(plugin.package)) return;

  const mode: InstallMode = isInstalled(plugin) ? "update" : "install";
  installingPackages.value = [...installingPackages.value, plugin.package];
  installingModes.value = { ...installingModes.value, [plugin.package]: mode };

  try {
    await useInstallDshPlugin(plugin.command).start();
    installedPlugins.value = await useDshPluginList();
    toast.success(`${plugin.title}${mode === "install" ? "安装" : "更新"}成功`);
  } catch (cause) {
    logger.warn("recommendedPlugins", `${plugin.title} ${mode === "install" ? "安装" : "更新"}失败:`, cause);
    toast.error(getErrorMessage(cause));
  } finally {
    installingPackages.value = installingPackages.value.filter((packageName) => packageName !== plugin.package);
    const nextModes = { ...installingModes.value };
    delete nextModes[plugin.package];
    installingModes.value = nextModes;
  }
}

async function open(): Promise<void> {
  dialogEl.value?.showModal();
  if (loadState.value.kind === "idle" || loadState.value.kind === "error") {
    void loadPlugins();
  } else {
    try {
      installedPlugins.value = await useDshPluginList();
    } catch (cause) {
      logger.warn("recommendedPlugins", "刷新本地插件列表失败:", cause);
      toast.error(getErrorMessage(cause));
    }
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
