<script setup lang="ts">
import { computed } from "vue";
import { useLocalStorage } from "@vueuse/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import IconWeb from "~icons/streamline-plump/web";
import IconLog from "~icons/ix/log";
import IconInstall from "~icons/clarity/install-line";
import type { RuntimeSnapshot } from "../../composables/useRuntime";
import { busy, installDsh, openLogDir, phase } from "../../composables/useRuntime";
import { beginAction, reportActionError } from "../../composables/useActionFeedback";

const LINKS = {
  nodejs: "https://nodejs.org/zh-cn/download",
} as const;

const { state } = defineProps<{ state: RuntimeSnapshot }>();
const installing = computed(() => phase.value === "installing");
const environmentReady = computed(() => Boolean(state.node && state.npm));
const useMirror = useLocalStorage("dsh-use-mirror", false);

const selectedRemote = computed(() => (useMirror.value ? state.remoteMirror : state.remote));
const selectedSourceFailed = computed(() =>
  useMirror.value ? state.versionChecked && state.remoteMirror === null : state.versionError,
);
const selectedVersionReady = computed(
  () => state.versionChecked && !selectedSourceFailed.value && selectedRemote.value !== null,
);
const installUpdateDisabled = computed(() => {
  if (busy.value || !environmentReady.value || !selectedVersionReady.value) return true;
  return !(state.local === null || state.local !== selectedRemote.value);
});

const installUpdateLabel = computed(() => {
  if (installing.value) {
    return state.local ? "正在更新…" : "正在安装…";
  }
  return state.local === null ? "安装 DSH" : "更新 DSH";
});

function openExternal(url: string) {
  void openUrl(url);
}

function startInstall() {
  if (installUpdateDisabled.value) return;
  beginAction();
  void installDsh(useMirror.value).catch(reportActionError);
}
</script>

<template>
  <section class="min-w-0 p-8 lg:p-10" aria-labelledby="actions-title">
    <h2 id="actions-title" class="text-base-content text-xl font-black">快捷操作</h2>

    <div class="mt-8 space-y-3">
      <button
        type="button"
        class="btn btn-outline btn-primary btn-block min-h-12 justify-start gap-2 rounded-xl px-4 font-bold"
        @click="openExternal(LINKS.nodejs)"
      >
        <IconWeb class="h-5 w-5" />
        Node.js 官网
      </button>

      <button
        type="button"
        class="btn btn-outline btn-primary btn-block min-h-12 justify-start gap-2 rounded-xl px-4 font-bold"
        @click="openLogDir"
      >
        <IconLog class="h-5 w-5" />
        查看日志
      </button>
      <div class="divider"></div>
      <button
        type="button"
        class="btn btn-block min-h-12 justify-start gap-2 rounded-xl px-4 font-bold disabled:opacity-100"
        :class="installUpdateDisabled ? 'btn-outline btn-neutral' : 'btn-outline btn-primary'"
        :disabled="installUpdateDisabled"
        :aria-busy="installing"
        @click="startInstall"
      >
        <span v-if="installing" class="loading loading-spinner loading-sm" aria-label="正在安装"></span>
        <IconInstall v-else class="h-5 w-5" />
        {{ installUpdateLabel }}
      </button>
      <label class="label text-base-content/70 mt-2 cursor-pointer justify-start gap-2 px-0 text-base">
        <input v-model="useMirror" type="checkbox" class="checkbox checkbox-primary" :disabled="busy" />
        <span>使用镜像源</span>
      </label>
      <p v-if="installUpdateDisabled && selectedSourceFailed" class="text-error mt-1 text-xs font-medium">
        {{ useMirror ? "镜像源版本查询失败，可取消勾选改用官方源。" : "官方源版本查询失败，可勾选使用镜像源。" }}
      </p>
    </div>
  </section>
</template>
