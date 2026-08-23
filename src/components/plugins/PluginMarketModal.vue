<template>
  <dialog ref="dialogEl" class="modal">
    <div class="modal-box">
      <h3 class="text-lg font-bold">插件市场</h3>
      <p class="py-4 text-base-content/70">插件市场内容待实现</p>
      <div class="modal-action">
        <form method="dialog">
          <button class="btn" type="submit">关闭</button>
        </form>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button>close</button></form>
  </dialog>
</template>

<script setup lang="ts">
import { ref } from "vue";

const dialogEl = ref<HTMLDialogElement | null>(null);

function open(): void {
  dialogEl.value?.showModal();
}

defineExpose({ open });
</script>
