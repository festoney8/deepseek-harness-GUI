<template>
  <section class="card card-border bg-base-100 shadow-sm">
    <div class="card-body gap-5">
      <h2 class="card-title">其他操作</h2>
      <div class="grid gap-3 sm:grid-cols-2">
        <button type="button" class="btn btn-outline" @click="pluginMarketModal?.open()">插件市场</button>
        <button type="button" class="btn btn-outline" @click="pluginInstallModal?.open()">安装插件</button>
        <button type="button" class="btn btn-outline" @click="openNodeDownload">安装 Node.js</button>
        <button type="button" class="btn btn-outline" @click="openLogsPanel">查看日志</button>
      </div>
    </div>
    <PluginMarketModal ref="pluginMarketModal" />
    <PluginInstallModal ref="pluginInstallModal" />
  </section>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { openLogs } from "../../ipc/ipc";
import PluginInstallModal from "../plugins/PluginInstallModal.vue";
import PluginMarketModal from "../plugins/PluginMarketModal.vue";
import { getErrorMessage, useToast } from "../../composables/useToast";
import { logger } from "../../utils/log";

const toast = useToast();
const pluginMarketModal = ref<InstanceType<typeof PluginMarketModal> | null>(null);
const pluginInstallModal = ref<InstanceType<typeof PluginInstallModal> | null>(null);

async function openNodeDownload(): Promise<void> {
  try {
    await openUrl("https://nodejs.org/en/download");
  } catch (error) {
    logger.warn("openUrl", "打开外部链接失败:", "https://nodejs.org/en/download", error);
    toast.error(getErrorMessage(error));
  }
}

async function openLogsPanel(): Promise<void> {
  try {
    await openLogs();
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}
</script>
