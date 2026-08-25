<template>
  <div class="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3 border-b border-base-200 py-2 last:border-b-0">
    <span class="text-base text-base-content/75">{{ label }}</span>
    <button
      v-if="state.kind === 'error'"
      class="btn btn-primary btn-outline btn-xs text-sm"
      type="button"
      @click="openDownload"
    >
      去下载
    </button>
    <span
      v-else
      class="badge max-w-48 truncate text-right text-base font-medium"
      :class="accent ? 'badge-accent badge-soft' : 'badge-ghost'"
      :title="displayedValue"
    >
      {{ displayedValue }}
    </span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { displayVersion, type VersionState } from "@/stores/env";
import { logger } from "@/utils/log";

const props = defineProps<{ label: string; state: VersionState; accent?: boolean; errorHref: string }>();
const displayedValue = computed(() => displayVersion(props.state));

function openDownload(): void {
  void openUrl(props.errorHref).catch((error) => {
    logger.warn("openUrl", "打开下载链接失败:", props.errorHref, error);
  });
}
</script>
