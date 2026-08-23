<template>
  <div class="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3 border-b border-base-200 py-2 last:border-b-0">
    <span class="text-base text-base-content/75">{{ label }}</span>
    <span
      class="badge max-w-48 truncate text-right text-base font-medium"
      :class="state.kind === 'error' || accent ? 'badge-accent badge-soft' : 'badge-ghost'"
      :title="displayedValue"
    >
      <a
        v-if="state.kind === 'error' && errorHref"
        class="link link-hover link-primary text-base font-medium"
        :title="errorHref"
        @click="openDownload"
        >去下载</a
      >
      <div v-else>{{ displayedValue }}</div>
    </span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { displayVersion, type VersionState } from "../../stores/env";
import { logger } from "../../utils/log";

const props = defineProps<{ label: string; state: VersionState; accent?: boolean; errorHref?: string }>();
const displayedValue = computed(() => displayVersion(props.state));

async function openDownload(): Promise<void> {
  if (!props.errorHref) return;
  openUrl(props.errorHref).catch((error) => {
    logger.warn("openUrl", "打开下载链接失败:", props.errorHref, error);
  });
}
</script>
