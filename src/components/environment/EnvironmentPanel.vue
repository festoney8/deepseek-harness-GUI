<template>
  <section class="card card-border h-full min-h-0 bg-base-100 shadow-sm">
    <div class="card-body gap-1">
      <div class="flex items-center justify-between gap-3">
        <h2 class="card-title">环境检查</h2>
        <button
          class="btn btn-square btn-ghost btn-md shrink-0"
          type="button"
          title="重新检查"
          aria-label="重新检查"
          :disabled="refreshing"
          @click="refreshEnv"
        >
          <RefreshIcon class="size-5" :class="{ 'animate-spin': refreshing }" aria-hidden="true" />
        </button>
      </div>
      <div class="grid gap-1">
        <fieldset v-for="(group, index) in groups" :key="group.title" class="fieldset gap-1">
          <legend v-if="index > 0" class="fieldset-legend text-base">{{ group.title }}</legend>
          <VersionRow
            v-for="row in group.rows"
            :key="row.label"
            :label="row.label"
            :state="row.state"
            :accent="row.accent"
          />
        </fieldset>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import RefreshIcon from "~icons/mynaui/refresh-solid";
import { useEnvStore, type VersionState } from "../../stores/env";
import VersionRow from "./VersionRow.vue";

const env = useEnvStore();
const refreshing = ref(false);

function normalizeVersion(version: string): string {
  return version.replace(/^v/, "");
}

function isLatestMismatch(state: VersionState): boolean {
  return (
    env.dshVer.kind === "ok" &&
    state.kind === "ok" &&
    normalizeVersion(env.dshVer.version) !== normalizeVersion(state.version)
  );
}

const groups = computed(() => [
  {
    title: "本地环境",
    rows: [
      { label: "本地 node", state: env.nodeVer, accent: false },
      { label: "本地 npm", state: env.npmVer, accent: false },
      { label: "本地 DSH", state: env.dshVer, accent: false },
    ],
  },
  {
    title: "最新 DSH 稳定版 (latest)",
    rows: [
      { label: "官方源", state: env.latestDshVer, accent: isLatestMismatch(env.latestDshVer) },
      { label: "镜像源", state: env.latestDshVerWithMirror, accent: isLatestMismatch(env.latestDshVerWithMirror) },
    ],
  },
  {
    title: "最新 DSH 测试版 (next)",
    rows: [
      { label: "官方源", state: env.nextDshVer, accent: isLatestMismatch(env.nextDshVer) },
      { label: "镜像源", state: env.nextDshVerWithMirror, accent: isLatestMismatch(env.nextDshVerWithMirror) },
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
