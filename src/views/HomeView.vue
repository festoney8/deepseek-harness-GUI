<template>
  <div class="min-h-screen overflow-x-auto bg-base-200 text-base-content">
    <AppHeader />
    <main
      class="mx-auto grid min-w-[1120px] max-w-[1440px] grid-cols-[minmax(20rem,1fr)_minmax(22rem,1.1fr)_minmax(22rem,1.1fr)] gap-5 px-6 pb-8 sm:px-8"
    >
      <EnvironmentPanel />
      <div class="grid content-start gap-5">
        <DshInstallActions />
        <OtherActionsCard />
      </div>
      <div class="grid content-start gap-5">
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
