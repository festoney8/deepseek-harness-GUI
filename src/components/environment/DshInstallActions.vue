<template>
  <section class="card card-border bg-base-100 shadow-sm">
    <div class="card-body gap-4">
      <h2 class="card-title">安装/更新 DSH</h2>
      <fieldset class="fieldset gap-2">
        <legend class="fieldset-legend text-base">稳定版 (latest)</legend>
        <div class="grid gap-3 sm:grid-cols-2">
          <button
            class="btn btn-primary btn-block"
            type="button"
            :disabled="latestOfficialDisabled"
            @click="installOfficialLatest"
          >
            <span v-if="officialLatest.running" class="loading loading-spinner loading-sm" aria-hidden="true"></span>
            {{ latestOfficialLabel }}
          </button>
          <button
            class="btn btn-primary btn-block"
            type="button"
            :disabled="latestMirrorDisabled"
            @click="installMirrorLatest"
          >
            <span v-if="mirrorLatest.running" class="loading loading-spinner loading-sm" aria-hidden="true"></span>
            {{ latestMirrorLabel }}
          </button>
        </div>
      </fieldset>
      <fieldset class="fieldset gap-2">
        <legend class="fieldset-legend text-base">测试版 (next)</legend>
        <div class="grid gap-3 sm:grid-cols-2">
          <button
            class="btn btn-primary btn-block"
            type="button"
            :disabled="nextOfficialDisabled"
            @click="installOfficialNext"
          >
            <span v-if="officialNext.running" class="loading loading-spinner loading-sm" aria-hidden="true"></span>
            {{ nextOfficialLabel }}
          </button>
          <button
            class="btn btn-primary btn-block"
            type="button"
            :disabled="nextMirrorDisabled"
            @click="installMirrorNext"
          >
            <span v-if="mirrorNext.running" class="loading loading-spinner loading-sm" aria-hidden="true"></span>
            {{ nextMirrorLabel }}
          </button>
        </div>
      </fieldset>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useDshStore } from "../../stores/dsh";
import { useEnvStore, type VersionState } from "../../stores/env";
import { getErrorMessage, useToast } from "../../composables/useToast";
import { useInstallDsh } from "../../composables/useInstallDsh";

const env = useEnvStore();
const dsh = useDshStore();
const toast = useToast();
const officialLatest = useInstallDsh(false, "latest");
const officialNext = useInstallDsh(false, "next");
const mirrorLatest = useInstallDsh(true, "latest");
const mirrorNext = useInstallDsh(true, "next");

function normalizeVersion(version: string): string {
  return version.replace(/^v/, "");
}

function isDifferent(remote: VersionState): boolean {
  return (
    env.dshVer.kind === "ok" &&
    remote.kind === "ok" &&
    normalizeVersion(env.dshVer.version) !== normalizeVersion(remote.version)
  );
}

function operationLabel(source: "official" | "mirror"): string {
  const action = env.dshVer.kind === "missing" || env.dshVer.kind === "error" ? "安装" : "更新";
  const sourceName = source === "official" ? "官方源" : "镜像源";
  return `${action}（${sourceName}）`;
}

function versionAvailable(remote: VersionState): boolean {
  if (remote.kind !== "ok") return false;
  if (env.dshVer.kind === "missing" || env.dshVer.kind === "error") return true;
  return isDifferent(remote);
}

const anyInstallRunning = computed(
  () =>
    officialLatest.running.value ||
    officialNext.running.value ||
    mirrorLatest.running.value ||
    mirrorNext.running.value,
);
const commonLocked = computed(
  () => anyInstallRunning.value || env.nodeVer.kind !== "ok" || env.npmVer.kind !== "ok" || dsh.phase !== "stopped",
);
const latestOfficialLabel = computed(() => operationLabel("official"));
const nextOfficialLabel = computed(() => operationLabel("official"));
const latestMirrorLabel = computed(() => operationLabel("mirror"));
const nextMirrorLabel = computed(() => operationLabel("mirror"));
const latestOfficialDisabled = computed(() => commonLocked.value || !versionAvailable(env.latestDshVer));
const nextOfficialDisabled = computed(() => commonLocked.value || !versionAvailable(env.nextDshVer));
const latestMirrorDisabled = computed(() => commonLocked.value || !versionAvailable(env.latestDshVerWithMirror));
const nextMirrorDisabled = computed(() => commonLocked.value || !versionAvailable(env.nextDshVerWithMirror));

async function runInstall(operation: ReturnType<typeof useInstallDsh>, tag: "latest" | "next"): Promise<void> {
  try {
    await operation.start();
    await env.getDshVer();
    toast.success(`${tag === "latest" ? "稳定版" : "测试版"}安装/更新完成`);
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}

const installOfficialLatest = () => runInstall(officialLatest, "latest");
const installOfficialNext = () => runInstall(officialNext, "next");
const installMirrorLatest = () => runInstall(mirrorLatest, "latest");
const installMirrorNext = () => runInstall(mirrorNext, "next");
</script>
