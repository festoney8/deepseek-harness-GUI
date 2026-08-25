<template>
  <section class="card card-border bg-base-100 shadow-sm">
    <div class="card-body gap-5">
      <h2 class="card-title">其他操作</h2>
      <div class="grid gap-3 sm:grid-cols-2">
        <button type="button" class="btn btn-outline" @click="pluginMarketModal?.open()">插件市场</button>
        <button type="button" class="btn btn-outline" @click="recommendedPluginsModal?.open()">DSH 插件推荐</button>
        <button type="button" class="btn btn-outline" @click="pluginInstallModal?.open()">安装插件</button>
        <button type="button" class="btn btn-outline" @click="openLogsPanel">查看日志</button>
      </div>
    </div>
    <PluginMarketModal ref="pluginMarketModal" />
    <PluginInstallModal ref="pluginInstallModal" />
    <RecommendedPluginsModal ref="recommendedPluginsModal" />
  </section>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { openLogs } from "@/ipc/ipc";
import PluginInstallModal from "../plugins/PluginInstallModal.vue";
import PluginMarketModal from "../plugins/PluginMarketModal.vue";
import RecommendedPluginsModal from "../plugins/RecommendedPluginsModal.vue";
import { getErrorMessage, useToast } from "@/composables/useToast";

const toast = useToast();
const pluginMarketModal = ref<InstanceType<typeof PluginMarketModal> | null>(null);
const pluginInstallModal = ref<InstanceType<typeof PluginInstallModal> | null>(null);
const recommendedPluginsModal = ref<InstanceType<typeof RecommendedPluginsModal> | null>(null);

async function openLogsPanel(): Promise<void> {
  try {
    await openLogs();
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}
</script>
