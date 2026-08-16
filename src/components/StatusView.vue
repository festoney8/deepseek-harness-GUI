<script setup lang="ts">
import type { RuntimeSnapshot } from "../composables/useRuntime";
import { useReleaseInfo } from "../composables/useReleaseInfo";
import ActionsPanel from "./status/ActionsPanel.vue";
import ConnectionPanel from "./status/ConnectionPanel.vue";
import EnvironmentPanel from "./status/EnvironmentPanel.vue";
import StatusHeader from "./status/StatusHeader.vue";

const { state } = defineProps<{ state: RuntimeSnapshot }>();
const { appVersion, versionOutdated } = useReleaseInfo();
</script>

<template>
  <div class="dsh-status-view text-base-content h-full overflow-y-auto px-8 py-8">
    <div class="mx-auto flex min-h-full w-full max-w-6xl flex-col justify-center gap-10">
      <StatusHeader :app-version="appVersion" :version-outdated="versionOutdated" />

      <section
        class="dsh-panel card card-border grid min-h-0 grid-cols-[minmax(0,44fr)_auto_minmax(0,24fr)_auto_minmax(0,32fr)] overflow-hidden rounded-3xl"
        aria-label="DeepSeek Harness 控制面板"
      >
        <EnvironmentPanel :state="state" />
        <div class="divider divider-horizontal m-0 h-auto"></div>
        <ActionsPanel :state="state" />
        <div class="divider divider-horizontal m-0 h-auto"></div>
        <ConnectionPanel :state="state" />
      </section>
    </div>
  </div>
</template>
