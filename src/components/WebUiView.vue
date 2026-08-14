<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ port: number }>();
const src = computed(() => `http://127.0.0.1:${props.port}/`);
</script>

<template>
  <iframe :src="src" class="h-full w-full border-0 bg-white" />
</template>
