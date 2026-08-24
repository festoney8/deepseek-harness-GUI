<template>
  <div role="alert" class="alert w-full" :class="alertClass">
    <span class="min-w-0 flex-1 truncate">{{ toast.message }}</span>
    <button
      type="button"
      class="btn btn-ghost btn-square btn-xs shrink-0"
      aria-label="关闭通知"
      @click="$emit('dismiss')"
    >
      <CloseIcon class="size-4" aria-hidden="true" />
    </button>
  </div>
</template>

<script setup lang="ts">
import CloseIcon from "~icons/material-symbols/close";
import type { ToastItem } from "../../composables/useToast";

const props = defineProps<{ toast: ToastItem }>();
defineEmits<{ dismiss: [] }>();
const alertClass = `alert-${props.toast.kind}`;
</script>
