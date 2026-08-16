<script setup lang="ts">
import type { RuntimeSnapshot } from "../composables/useRuntime";
import { useReleaseInfo } from "../composables/useReleaseInfo";
import ActionsPanel from "./status/ActionsPanel.vue";
import EnvironmentPanel from "./status/EnvironmentPanel.vue";
import StatusHeader from "./status/StatusHeader.vue";

const { state } = defineProps<{ state: RuntimeSnapshot }>();
const { appVersion, latestVersion, versionOutdated } = useReleaseInfo();
</script>

<template>
  <div
    class="h-full overflow-y-auto bg-[radial-gradient(circle_at_top,#eff6ff_0,#ffffff_42%,#f8fafc_100%)] px-8 py-8 text-slate-900 dark:bg-[radial-gradient(circle_at_top,#0f172a_0,#020617_42%,#0b1220_100%)] dark:text-slate-100"
  >
    <div class="mx-auto flex min-h-full w-full max-w-5xl flex-col justify-center gap-10">
      <StatusHeader :app-version="appVersion" :latest-version="latestVersion" :version-outdated="versionOutdated" />

      <section
        class="grid overflow-hidden rounded-3xl border border-blue-200 bg-white/95 shadow-[0_24px_70px_-35px_rgba(37,99,235,0.5)] md:grid-cols-[1.05fr_0.95fr] dark:border-blue-900 dark:bg-slate-900/95 dark:shadow-[0_24px_70px_-35px_rgba(2,6,23,0.9)]"
      >
        <EnvironmentPanel :state="state" />
        <ActionsPanel :state="state" />
      </section>
    </div>
  </div>
</template>
