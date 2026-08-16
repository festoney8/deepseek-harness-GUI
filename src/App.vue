<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import TitleBar from "./components/TitleBar.vue";
import StatusView from "./components/StatusView.vue";
import WebUiView from "./components/WebUiView.vue";
import { closeRequested, disposeRuntime, exitApp, hideToTray, initRuntime, state } from "./composables/useRuntime";
import { clearActionError, useActionFeedback } from "./composables/useActionFeedback";
import { disposeTheme, initTheme } from "./composables/useTheme";

const exitDialog = ref<HTMLDialogElement | null>(null);
const { actionError } = useActionFeedback();

watch(closeRequested, (requested) => {
  const dialog = exitDialog.value;
  if (!dialog) return;
  if (requested && !dialog.open) {
    dialog.showModal();
  } else if (!requested && dialog.open) {
    dialog.close();
  }
});

function cancelClose() {
  closeRequested.value = false;
}

function hideInTray() {
  hideToTray();
  closeRequested.value = false;
}

function requestExit() {
  void exitApp();
}

onMounted(initRuntime);
onMounted(initTheme);
onUnmounted(disposeRuntime);
onUnmounted(disposeTheme);
</script>

<template>
  <div class="bg-base-100 text-base-content flex h-full flex-col">
    <TitleBar />
    <main class="min-h-0 flex-1">
      <WebUiView v-if="state.phase === 'ready' && state.url" :url="state.url" />
      <StatusView v-else :state="state" />
    </main>

    <div v-if="actionError" class="toast toast-bottom toast-center z-50 mb-5">
      <div role="alert" class="alert alert-error rounded-xl shadow-xl">
        <span class="max-w-[min(32rem,calc(100vw-5rem))] break-words">{{ actionError }}</span>
        <button
          type="button"
          class="btn btn-ghost btn-sm rounded-xl"
          aria-label="关闭错误通知"
          @click="clearActionError"
        >
          关闭
        </button>
      </div>
    </div>

    <dialog ref="exitDialog" class="modal modal-middle" aria-labelledby="exit-dialog-title" @cancel.prevent>
      <div class="modal-box rounded-2xl">
        <h2 id="exit-dialog-title" class="text-lg font-bold">退出 DeepSeek Harness？</h2>
        <p class="text-base-content/70 mt-3 text-sm">请选择“最小化到托盘”以保留后台进程，或确认退出应用。</p>
        <div class="modal-action mt-8">
          <button type="button" class="btn btn-ghost rounded-xl" @click="cancelClose">取消</button>
          <button type="button" class="btn btn-outline btn-primary rounded-xl" @click="hideInTray">最小化到托盘</button>
          <button type="button" class="btn btn-error rounded-xl" @click="requestExit">退出</button>
        </div>
      </div>
    </dialog>
  </div>
</template>
