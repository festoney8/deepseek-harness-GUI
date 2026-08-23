<template>
  <dialog ref="dialogEl" class="modal">
    <div class="modal-box relative">
      <h3 class="mb-4 text-lg font-bold">安装插件</h3>
      <form method="dialog" class="absolute right-2 top-2">
        <button class="btn btn-sm btn-circle btn-ghost" type="submit" aria-label="关闭">
          <CloseIcon class="size-4" aria-hidden="true" />
        </button>
      </form>
      <p class="py-4 text-base-content/70">安装插件内容待实现</p>
    </div>
    <form method="dialog" class="modal-backdrop"><button>close</button></form>
  </dialog>
</template>

<script setup lang="ts">
import { ref } from "vue";
import CloseIcon from "~icons/material-symbols/close";

const dialogEl = ref<HTMLDialogElement | null>(null);

function open(): void {
  dialogEl.value?.showModal();
}

defineExpose({ open });
</script>
