<template>
  <div class="flex h-screen min-h-screen flex-col overflow-x-auto bg-base-200 text-base-content">
    <AppHeader />
    <main
      class="mx-auto grid h-2/3 min-h-0 min-w-280 max-w-360 flex-none grid-cols-[minmax(20rem,1fr)_minmax(22rem,1.1fr)_minmax(22rem,1.1fr)] items-stretch gap-5 overflow-y-auto px-6 pb-8 sm:px-8"
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
