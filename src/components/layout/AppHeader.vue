<template>
  <header class="hero bg-transparent py-8 sm:py-10">
    <div class="hero-content flex-col gap-4 text-center sm:flex-row sm:gap-6">
      <img :src="logo" alt="DeepSeek Harness GUI logo" class="size-24 object-contain sm:size-28" />
      <div>
        <h1 class="text-4xl font-bold tracking-tight text-base-content sm:text-5xl">DeepSeek Harness GUI</h1>
        <div class="mt-3 flex flex-wrap items-center justify-center gap-2 text-sm sm:justify-start">
          <button type="button" class="btn btn-ghost btn-sm" @click="openExternal(PROJECT_URL)">项目 GitHub</button>
          <span class="text-base-content/40" aria-hidden="true">|</span>
          <span class="text-base-content/70">版本：{{ appVersion }}</span>
          <button v-if="hasUpdate" type="button" class="btn btn-link btn-sm" @click="openExternal(RELEASES_URL)">
            有更新
          </button>
        </div>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed } from "vue";
import logo from "../../assets/logo.png";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEnvStore, displayVersion } from "../../stores/env";
import { getErrorMessage, useToast } from "../../composables/useToast";

const PROJECT_URL = "https://github.com/festoney8/deepseek-harness-GUI/";
const RELEASES_URL = "https://github.com/festoney8/deepseek-harness-GUI/releases";
const env = useEnvStore();
const toast = useToast();

function normalizedVersion(version: string): string {
  return version.replace(/^v/, "");
}

const appVersion = computed(() => displayVersion(env.appVer));
const hasUpdate = computed(
  () =>
    env.latestAppVer.kind === "ok" &&
    env.appVer.kind === "ok" &&
    normalizedVersion(env.latestAppVer.version) !== normalizedVersion(env.appVer.version),
);

async function openExternal(url: string): Promise<void> {
  try {
    await openUrl(url);
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}
</script>
