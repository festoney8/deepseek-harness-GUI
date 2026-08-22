<template>
  <section class="card card-border bg-base-100 shadow-sm">
    <div class="card-body gap-5">
      <h2 class="card-title">其他操作</h2>
      <div class="grid gap-3 sm:grid-cols-2">
        <button type="button" class="btn btn-outline" @click="noop">插件市场</button>
        <button type="button" class="btn btn-outline" @click="noop">安装插件</button>
        <button type="button" class="btn btn-outline" @click="openNodeDownload">安装 Node.js</button>
        <button type="button" class="btn btn-outline" @click="openLogsPanel">查看日志</button>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import { openLogs } from "../../ipc/ipc";
import { getErrorMessage, useToast } from "../../composables/useToast";

const toast = useToast();

function noop(): void {
  // 占位操作暂不执行任何动作。
}

async function openNodeDownload(): Promise<void> {
  try {
    await openUrl("https://nodejs.org/en/download");
  } catch (error) {
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
