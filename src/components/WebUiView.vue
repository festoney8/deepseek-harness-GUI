<script setup lang="ts">
const { url } = defineProps<{ url: string }>();
</script>

<template>
  <iframe
    :src="url"
    allow="camera *; clipboard-read *; clipboard-write *; downloads *; fullscreen *; microphone *; speaker-selection *"
    class="bg-base-100 h-full w-full border-0"
  />
</template>
