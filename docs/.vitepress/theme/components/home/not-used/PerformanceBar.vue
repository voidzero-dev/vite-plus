<script setup lang="ts">
import { ProgressIndicator, ProgressRoot } from 'reka-ui';

import type { PerformanceData } from '../../../data/performance';
interface Props {
  data: PerformanceData;
  backgroundImage?: string;
}
const { data, backgroundImage = '@local-assets/backgrounds/vite.jpg' } = defineProps<Props>();
</script>

<template>
  <div class="flex items-center w-full gap-3">
    <p
      :class="[
        'font-mono text-xs uppercase tracking-wide w-24 md:w-36 shrink-0',
        data.isPrimary ? 'text-white' : 'text-grey',
      ]"
    >
      {{ data.name }}
    </p>
    <ProgressRoot class="rounded-xs relative h-3 overflow-hidden bg-slate w-full">
      <ProgressIndicator
        :class="[
          'indicator rounded-xs block relative h-full bg-center transition-transform overflow-hidden duration-[660ms] ease-[cubic-bezier(0.65, 0, 0.35, 1)]',
          data.isPrimary ? `bg-wine bg-[url(${backgroundImage})]` : 'bg-grey',
        ]"
        :style="`width: ${data.percentage}%`"
      />
    </ProgressRoot>
    <p
      :class="[
        'font-mono text-xs uppercase tracking-wide shrink-0',
        data.isPrimary ? 'text-white' : 'text-grey',
      ]"
    >
      / {{ data.time }}
    </p>
  </div>
</template>
