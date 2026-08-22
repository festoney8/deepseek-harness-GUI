<template>
  <div class="toast toast-end toast-bottom z-50 w-[min(24rem,calc(100vw-2rem))]">
    <AppToast v-for="toast in toasts" :key="toast.id" :toast="toast" @dismiss="dismiss(toast.id)" />
  </div>
</template>

<script setup lang="ts">
import AppToast from "./AppToast.vue";
import { useToast } from "../../composables/useToast";

const { toasts, dismiss } = useToast();
</script>
