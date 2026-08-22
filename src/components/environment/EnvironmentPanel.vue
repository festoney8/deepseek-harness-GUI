<template>
  <section class="card card-border h-fit self-start bg-base-100 shadow-sm">
    <div class="card-body gap-1">
      <div class="flex items-center justify-between gap-3">
        <h2 class="card-title">环境检查</h2>
        <button class="btn btn-ghost btn-md shrink-0" type="button" :disabled="refreshing" @click="refreshEnv">
          <span v-if="refreshing" class="loading loading-spinner loading-sm" aria-hidden="true"></span>
          重新检查
        </button>
      </div>
      <div class="grid gap-1">
        <fieldset v-for="(group, index) in groups" :key="group.title" class="fieldset gap-1">
          <legend v-if="index > 0" class="fieldset-legend text-base">{{ group.title }}</legend>
          <VersionRow v-for="row in group.rows" :key="row.label" :label="row.label" :state="row.state" />
        </fieldset>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useEnvStore } from "../../stores/env";
import VersionRow from "./VersionRow.vue";

const env = useEnvStore();
const refreshing = ref(false);

const groups = computed(() => [
  {
    title: "本地环境",
    rows: [
      { label: "本地 node", state: env.nodeVer },
      { label: "本地 npm", state: env.npmVer },
      { label: "本地 DSH", state: env.dshVer },
    ],
  },
  {
    title: "最新 DSH 稳定版 (latest)",
    rows: [
      { label: "官方源", state: env.latestDshVer },
      { label: "镜像源", state: env.latestDshVerWithMirror },
    ],
  },
  {
    title: "最新 DSH 测试版 (next)",
    rows: [
      { label: "官方源", state: env.nextDshVer },
      { label: "镜像源", state: env.nextDshVerWithMirror },
    ],
  },
]);

async function refreshEnv(): Promise<void> {
  if (refreshing.value) return;
  refreshing.value = true;
  try {
    await env.refreshAllVersions();
  } finally {
    refreshing.value = false;
  }
}
</script>
