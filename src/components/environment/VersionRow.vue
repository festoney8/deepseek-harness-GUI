<template>
  <div class="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3 border-b border-base-200 py-2 last:border-b-0">
    <span class="text-base text-base-content/75">{{ label }}</span>
    <span
      class="badge max-w-48 truncate text-right text-base font-medium"
      :class="state.kind === 'error' || accent ? 'badge-accent badge-soft' : 'badge-ghost'"
      :title="displayedValue"
    >
      {{ displayedValue }}
    </span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { displayVersion, type VersionState } from "../../stores/env";

const props = defineProps<{ label: string; state: VersionState; accent?: boolean }>();
const displayedValue = computed(() => displayVersion(props.state));
</script>
