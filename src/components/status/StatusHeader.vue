<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import IconLink from "~icons/line-md/link";
import logoUrl from "../../assets/logo.png";

const LINKS = {
  marketplace: "https://dshfind.com/zh",
  github: "https://github.com/festoney8/deepseek-harness-GUI",
  releases: "https://github.com/festoney8/deepseek-harness-GUI/releases",
} as const;

const { appVersion, latestVersion, versionOutdated } = defineProps<{
  appVersion: string;
  latestVersion: string;
  versionOutdated: boolean;
}>();

function openExternal(url: string) {
  void openUrl(url);
}
</script>

<template>
  <header class="flex items-center justify-center gap-8">
    <div class="relative shrink-0">
      <div class="absolute inset-4 rounded-full bg-blue-400/20 blur-2xl" aria-hidden="true"></div>
      <img :src="logoUrl" alt="DeepSeek Harness GUI logo" class="relative h-36 w-36 object-contain drop-shadow-xl" />
    </div>

    <div class="min-w-0">
      <h1 class="text-4xl font-black text-[#315d9c] lg:text-5xl dark:text-blue-200">DeepSeek Harness GUI</h1>
      <nav class="mt-5 ml-1 flex items-center gap-4" aria-label="项目链接">
        <button
          type="button"
          class="group inline-flex cursor-pointer items-center gap-1.5 text-base font-bold text-[#315d9c] transition hover:text-blue-500 focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:ring-offset-4 focus-visible:outline-none dark:text-blue-200 dark:hover:text-blue-100"
          @click="openExternal(LINKS.marketplace)"
        >
          插件市场
          <IconLink class="h-4 w-4 transition group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
        </button>
        <span class="h-5 w-px bg-blue-200 dark:bg-blue-900" aria-hidden="true"></span>
        <button
          type="button"
          class="group inline-flex cursor-pointer items-center gap-1.5 text-base font-bold text-[#315d9c] transition hover:text-blue-500 focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:ring-offset-4 focus-visible:outline-none dark:text-blue-200 dark:hover:text-blue-100"
          @click="openExternal(LINKS.github)"
        >
          项目 GitHub
          <IconLink class="h-4 w-4 transition group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
        </button>
        <span class="h-5 w-px bg-blue-200 dark:bg-blue-900" aria-hidden="true"></span>
        <button
          type="button"
          class="group inline-flex cursor-pointer items-center gap-1.5 text-base font-bold transition focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:ring-offset-4 focus-visible:outline-none"
          :class="
            versionOutdated
              ? 'text-amber-600 hover:text-amber-700 dark:text-amber-400 dark:hover:text-amber-300'
              : 'text-[#315d9c] hover:text-blue-500 dark:text-blue-200 dark:hover:text-blue-100'
          "
          @click="openExternal(LINKS.releases)"
        >
          最新 {{ latestVersion }}
          <IconLink class="h-4 w-4 transition group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
        </button>
        <span class="h-5 w-px bg-blue-200 dark:bg-blue-900" aria-hidden="true"></span>
        <span class="text-base font-bold text-[#315d9c] dark:text-blue-200">当前 v{{ appVersion }}</span>
      </nav>
    </div>
  </header>
</template>
