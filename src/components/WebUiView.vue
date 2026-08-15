<script setup lang="ts">
const { url } = defineProps<{ url: string }>();
</script>

<template>
  <iframe
    :src="url"
    allow="camera *; clipboard-read *; clipboard-write *; downloads *; fullscreen *; microphone *; speaker-selection *"
    class="h-full w-full border-0 bg-white dark:bg-slate-900"
  />
</template>
