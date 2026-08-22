<template>
  <section class="card card-border bg-base-100 shadow-sm">
    <div class="card-body gap-4">
      <h2 class="card-title">远程连接</h2>
      <form class="grid gap-3" @submit.prevent="connect">
        <fieldset class="fieldset grid grid-cols-[3rem_minmax(0,1fr)] items-center gap-2">
          <label class="fieldset-legend justify-center text-base" for="remote-host">主机</label>
          <input
            id="remote-host"
            v-model="host"
            class="input validator w-full focus:outline-none! focus:ring-0!"
            type="text"
            placeholder="127.0.0.1"
            autocomplete="off"
            required
            :pattern="HOST_PATTERN"
            :class="!validHost ? 'input-error border-2 border-error focus:border-error' : ''"
          />
        </fieldset>
        <div class="grid grid-cols-[minmax(0,1.8fr)_minmax(0,1.2fr)] gap-4">
          <fieldset class="fieldset grid grid-cols-[3rem_minmax(0,1fr)] items-center gap-2">
            <label class="fieldset-legend justify-center text-base" for="remote-port">端口</label>
            <input
              id="remote-port"
              v-model="remotePort"
              class="input validator w-full focus:outline-none! focus:ring-0!"
              type="text"
              inputmode="numeric"
              autocomplete="off"
              required
              pattern="[0-9]+"
              min="1"
              max="65535"
              :class="!validRemotePort ? 'input-error border-2 border-error focus:border-error' : ''"
            />
          </fieldset>
          <fieldset class="fieldset grid grid-cols-[3rem_minmax(0,1fr)] items-center gap-2">
            <label class="fieldset-legend justify-center text-base" for="remote-protocol">协议</label>
            <select
              id="remote-protocol"
              v-model="protocol"
              class="select validator w-full focus:outline-none! focus:ring-0! focus:shadow-none! focus-within:outline-none! focus-within:shadow-none! open:outline-none! open:shadow-none!"
              required
            >
              <option value="http">http</option>
              <option value="https">https</option>
            </select>
          </fieldset>
        </div>
        <button class="btn btn-primary btn-outline btn-block mt-2" type="submit" :disabled="!validRemote || connecting">
          <span v-if="connecting" class="loading loading-spinner loading-sm" aria-hidden="true"></span>
          {{ connecting ? "连接中…" : "远程连接 DSH" }}
        </button>
      </form>
    </div>
  </section>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { connectRemote, createWindowWithUrl } from "../../ipc/ipc";
import { getErrorMessage, useToast } from "../../composables/useToast";
import { useConnectionForm } from "../../composables/useConnectionForm";

const form = useConnectionForm();
const { protocol, host, remotePort, validHost, validRemotePort, validRemote, normalizedHost, remotePortNumber } = form;
const toast = useToast();
const connecting = ref(false);
const HOST_PATTERN = "(?:localhost|(?:[0-9]{1,3}\\.){3}[0-9]{1,3})";

async function connect(): Promise<void> {
  form.markRemoteAttempted();
  if (!form.validRemote.value || connecting.value) return;
  form.saveRemote();
  connecting.value = true;
  try {
    const address = await connectRemote(protocol.value, normalizedHost.value, remotePortNumber.value);
    await createWindowWithUrl(address);
    toast.success("远程连接成功");
  } catch (error) {
    toast.error(getErrorMessage(error));
  } finally {
    connecting.value = false;
  }
}
</script>
