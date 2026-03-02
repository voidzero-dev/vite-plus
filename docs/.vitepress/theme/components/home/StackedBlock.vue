<script setup lang="ts">
defineProps<{
  src: string;
  alt: string;
  href?: string;
}>();

import {
  TooltipArrow,
  TooltipContent,
  TooltipPortal,
  TooltipProvider,
  TooltipRoot,
  TooltipTrigger,
} from 'reka-ui';
</script>

<template>
  <TooltipProvider :delayDuration="0">
    <TooltipRoot>
      <TooltipTrigger as-child>
        <li v-if="!href">
          <img loading="lazy" :src="src" :alt="alt" />
        </li>
      </TooltipTrigger>
      <TooltipTrigger as-child v-if="href">
        <a :href="href" target="_blank" rel="noopener noreferrer">
          <img loading="lazy" :src="src" :alt="alt" />
        </a>
      </TooltipTrigger>
      <TooltipPortal>
        <TooltipContent
          class="data-[state=delayed-open]:data-[side=top]:animate-slideDownAndFade data-[state=delayed-open]:data-[side=right]:animate-slideLeftAndFade data-[state=delayed-open]:data-[side=left]:animate-slideRightAndFade data-[state=delayed-open]:data-[side=bottom]:animate-slideUpAndFade select-none font-mono text-space text-sm px-1.5 py-0.5 outline outline-stroke rounded will-change-[transform,opacity]"
          :side-offset="5"
        >
          {{ alt }}
          <TooltipArrow class="fill-white stroke-stroke" :width="12" :height="6" />
        </TooltipContent>
      </TooltipPortal>
    </TooltipRoot>
  </TooltipProvider>
</template>

<style scoped></style>
