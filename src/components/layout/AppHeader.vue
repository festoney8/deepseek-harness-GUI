<template>
  <header class="hero min-h-0 h-full bg-transparent">
    <div class="hero-content flex-col gap-4 text-center sm:flex-row sm:gap-6">
      <img :src="logo" alt="logo" class="size-40 object-contain" />
      <div>
        <h1 class="text-4xl font-bold tracking-tight text-[#325d9d] sm:text-5xl">DeepSeek Harness GUI</h1>
        <div class="mt-6 flex flex-wrap items-center justify-center gap-2 text-sm sm:justify-start">
          <a class="text-base link link-hover font-bold text-[#325d9d]" @click="openExternal(PROJECT_URL)"
            >项目 GitHub</a
          >
          <span class="h-5 w-px bg-blue-200 dark:bg-blue-900 mx-2.5" aria-hidden="true"></span>
          <div class="text-base font-bold text-[#325d9d]">当前 v{{ appVersion }}</div>
          <span class="h-5 w-px bg-blue-200 dark:bg-blue-900 mx-2.5" aria-hidden="true"></span>
          <a
            v-if="hasUpdate"
            class="text-base link link-hover link-accent font-bold"
            @click="openExternal(RELEASES_URL)"
            >有更新 {{ latestAppVersion }}</a
          >
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
const latestAppVersion = computed(() => displayVersion(env.latestAppVer));
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
