<template>
  <div
    id="background"
    class="grid min-h-screen grid-rows-[minmax(min-content,1fr)_minmax(min-content,2fr)] overflow-x-auto bg-base-200 text-base-content"
  >
    <AppHeader />
    <main
      class="mx-auto grid min-w-280 max-w-360 grid-cols-[minmax(20rem,1fr)_minmax(22rem,1.1fr)_minmax(22rem,1.1fr)] items-stretch gap-5 px-6 pb-8 sm:px-8"
    >
      <EnvironmentPanel />
      <div class="grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)] gap-5">
        <DshInstallActions />
        <OtherActionsCard />
      </div>
      <div class="grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)] gap-5">
        <RemoteConnectPanel />
        <LocalDshPanel :installing="installing" />
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import AppHeader from "../components/layout/AppHeader.vue";
import EnvironmentPanel from "../components/environment/EnvironmentPanel.vue";
import DshInstallActions from "../components/environment/DshInstallActions.vue";
import OtherActionsCard from "../components/environment/OtherActionsCard.vue";
import RemoteConnectPanel from "../components/remote/RemoteConnectPanel.vue";
import LocalDshPanel from "../components/local/LocalDshPanel.vue";
import { isDshInstallRunning } from "../composables/useInstallDsh";

const installing = isDshInstallRunning;
</script>

<style lang="scss" scoped>
#background {
  background: radial-gradient(circle at top, #e8f1ff 0, #fcfcff 42%, #faf7ff 100%);
}
[data-theme="night"] {
  #background {
    background:
      radial-gradient(circle at 10% 15%, rgba(30, 64, 175, 0.28) 0%, rgba(30, 64, 175, 0.12) 25%, transparent 55%),
      radial-gradient(circle at 90% 10%, rgba(79, 70, 229, 0.22) 0%, rgba(79, 70, 229, 0.08) 30%, transparent 60%),
      radial-gradient(circle at 75% 80%, rgba(14, 116, 144, 0.18) 0%, transparent 55%),
      radial-gradient(circle at 25% 90%, rgba(67, 56, 202, 0.14) 0%, transparent 50%),
      linear-gradient(135deg, #020617 0%, #0b1120 30%, #111827 60%, #0f172a 100%);
  }
}
</style>
