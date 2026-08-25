<template>
  <div class="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3 border-b border-base-200 py-2 last:border-b-0">
    <span class="text-base text-base-content/75">{{ label }}</span>
    <button
      v-if="state.kind === 'error'"
      class="btn btn-primary btn-outline btn-xs text-sm"
      type="button"
      :disabled="installer.running.value"
      @click="installPnpm"
    >
      <span v-if="installer.running.value" class="loading loading-spinner loading-xs" aria-hidden="true" />
      {{ installer.running.value ? "安装中" : "安装" }}
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
import { useInstallPnpm } from "@/composables/useInstallPnpm";
import { useToast } from "@/composables/useToast";
import { useEnvStore, displayVersion, type VersionState } from "@/stores/env";
import { logger } from "@/utils/log";

const props = defineProps<{ label: string; state: VersionState; nodeState: VersionState; accent?: boolean }>();
const displayedValue = computed(() => displayVersion(props.state));
const env = useEnvStore();
const installer = useInstallPnpm();
const toast = useToast();

async function installPnpm(): Promise<void> {
  if (installer.running.value) return;
  if (props.nodeState.kind !== "ok") {
    toast.error("未检测到 Node.js，请先下载并安装 Node.js");
    return;
  }

  try {
    await installer.start();
    await env.getPnpmVer();
    toast.success("pnpm 安装成功");
  } catch (cause) {
    logger.warn("pnpm-install", "pnpm 安装失败，请查看日志:", cause);
    toast.error("pnpm 安装失败，请查看日志");
  }
}
</script>
