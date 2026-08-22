<template>
  <section class="card card-border bg-base-100 shadow-sm">
    <div class="card-body gap-4">
      <h2 class="card-title">本地启动</h2>
      <form class="grid gap-3" @submit.prevent="startLocal">
        <div class="grid grid-cols-[minmax(0,1.8fr)_minmax(0,1.2fr)] gap-4">
          <fieldset class="fieldset grid grid-cols-[3rem_minmax(0,1fr)] items-center gap-2">
            <label class="fieldset-legend justify-center text-base" for="local-host">主机</label>
            <input
              id="local-host"
              class="input w-full focus:outline-none! focus:ring-0!"
              type="text"
              value="127.0.0.1"
              disabled
            />
          </fieldset>
          <fieldset class="fieldset grid grid-cols-[3rem_minmax(0,1fr)] items-center gap-2">
            <label class="fieldset-legend justify-center text-base" for="local-port">端口</label>
            <input
              id="local-port"
              v-model="localPort"
              class="input validator w-full focus:outline-none! focus:ring-0!"
              type="text"
              inputmode="numeric"
              autocomplete="off"
              required
              pattern="[0-9]+"
              min="1"
              max="65535"
              :disabled="!isStopped"
              :class="!validLocalPort ? 'input-error border-2 border-error focus:border-error' : ''"
            />
          </fieldset>
        </div>
        <div v-if="isStopped" class="mt-2 grid gap-2">
          <button
            class="btn btn-primary btn-block"
            type="submit"
            :disabled="!validLocalPort || dsh.isBusy || installing"
          >
            <span v-if="dsh.isBusy" class="loading loading-spinner loading-sm" aria-hidden="true"></span>
            {{ dsh.isBusy ? "启动中…" : "本地运行 DSH" }}
          </button>
        </div>
        <div v-else class="grid gap-2 sm:grid-cols-3">
          <button class="btn btn-primary btn-outline" type="button" :disabled="dsh.isBusy" @click="openLocal">
            打开页面
          </button>
          <button class="btn btn-outline" type="button" :disabled="dsh.isBusy || installing" @click="restartLocal">
            <span
              v-if="dsh.phase === 'starting' || dsh.phase === 'stopping'"
              class="loading loading-spinner loading-sm"
              aria-hidden="true"
            ></span>
            重启
          </button>
          <button class="btn btn-error btn-outline" type="button" :disabled="dsh.isBusy" @click="stopLocal">
            <span v-if="dsh.phase === 'stopping'" class="loading loading-spinner loading-sm" aria-hidden="true"></span>
            终止
          </button>
        </div>
      </form>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { createWindowWithUrl } from "../../ipc/ipc";
import { useDshStore } from "../../stores/dsh";
import { getErrorMessage, useToast } from "../../composables/useToast";
import { useConnectionForm } from "../../composables/useConnectionForm";

const props = defineProps<{ installing?: boolean }>();
const form = useConnectionForm();
const { localPort, validLocalPort, localPortNumber } = form;
const dsh = useDshStore();
const toast = useToast();
const installing = computed(() => props.installing ?? false);
const isStopped = computed(() => dsh.phase === "stopped");

async function openLocal(): Promise<void> {
  if (!dsh.address) return;
  try {
    await createWindowWithUrl(dsh.address);
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}

async function startLocal(): Promise<void> {
  form.markLocalAttempted();
  if (!form.validLocalPort.value || !isStopped.value || installing.value) return;
  form.saveLocal();
  try {
    const address = await dsh.start(localPortNumber.value);
    await createWindowWithUrl(address);
    toast.success("DSH 已启动");
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}

async function restartLocal(): Promise<void> {
  if (!form.validLocalPort.value || installing.value) return;
  form.saveLocal();
  try {
    const address = await dsh.restart(localPortNumber.value);
    await createWindowWithUrl(address);
    toast.success("DSH 已重启");
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}

async function stopLocal(): Promise<void> {
  try {
    await dsh.stop();
    toast.success("DSH 已终止");
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}
</script>
