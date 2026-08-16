<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import IconLink from "~icons/line-md/link";
import logoUrl from "../../assets/logo.png";

const LINKS = {
  marketplace: "https://dshfind.com/zh",
  github: "https://github.com/festoney8/deepseek-harness-GUI",
  releases: "https://github.com/festoney8/deepseek-harness-GUI/releases",
} as const;

const { appVersion, versionOutdated } = defineProps<{
  appVersion: string;
  versionOutdated: boolean;
}>();

function openExternal(url: string) {
  void openUrl(url);
}
</script>

<template>
  <header class="hero bg-transparent">
    <div class="hero-content flex-row items-center justify-start gap-8 p-0">
      <div class="relative shrink-0">
        <div class="absolute inset-4 rounded-full bg-blue-400/20 blur-2xl" aria-hidden="true"></div>
        <img :src="logoUrl" alt="DeepSeek Harness GUI logo" class="relative h-36 w-36 object-contain drop-shadow-xl" />
      </div>

      <div class="min-w-0">
        <h1 class="dsh-hero-title text-4xl font-black lg:text-5xl">DeepSeek Harness GUI</h1>
        <nav class="mt-5 ml-1 flex flex-wrap items-center gap-x-4 gap-y-2" aria-label="项目链接">
          <button
            type="button"
            class="link link-primary link-hover inline-flex cursor-pointer items-center gap-1.5 text-base font-bold"
            @click="openExternal(LINKS.marketplace)"
          >
            插件市场
            <IconLink class="h-4 w-4" />
          </button>
          <span class="bg-primary/20 h-5 w-px" aria-hidden="true"></span>
          <button
            type="button"
            class="link link-primary link-hover inline-flex cursor-pointer items-center gap-1.5 text-base font-bold"
            @click="openExternal(LINKS.github)"
          >
            项目 GitHub
            <IconLink class="h-4 w-4" />
          </button>
          <span class="bg-primary/20 h-5 w-px" aria-hidden="true"></span>
          <span class="text-base-content/80 text-base font-bold">
            版本：v{{ appVersion || "正在加载…" }}
            <button
              v-if="versionOutdated"
              type="button"
              class="link link-primary link-hover ml-1 inline-flex items-center gap-1 font-bold whitespace-nowrap"
              @click="openExternal(LINKS.releases)"
            >
              有更新
              <IconLink class="ml-0.5 h-4 w-4" aria-hidden="true" />
            </button>
          </span>
        </nav>
      </div>
    </div>
  </header>
</template>
