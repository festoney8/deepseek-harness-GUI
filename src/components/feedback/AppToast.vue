<template>
  <div role="alert" class="alert w-full" :class="alertClass">
    <span class="min-w-0 flex-1 truncate">{{ toast.message }}</span>
    <button type="button" class="btn btn-ghost btn-xs shrink-0" aria-label="关闭通知" @click="$emit('dismiss')">
      关闭
    </button>
  </div>
</template>

<script setup lang="ts">
import type { ToastItem } from "../../composables/useToast";

const props = defineProps<{ toast: ToastItem }>();
defineEmits<{ dismiss: [] }>();
const alertClass = `alert-${props.toast.kind}`;
</script>
