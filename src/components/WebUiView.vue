<script setup lang="ts">
const { port } = defineProps<{ port: number }>();
</script>

<template>
  <iframe
    :src="`http://127.0.0.1:${port}/`"
    allow="
      camera *;
      clipboard-read *;
      clipboard-write *;
      downloads *;
      fullscreen *;
      microphone *;
      speaker-selection *;
    "
    class="h-full w-full border-0 bg-white"
  />
</template>
