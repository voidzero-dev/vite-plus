<script setup lang="ts">
import bunIcon from '@local-assets/icons/bun.png';
import denoIcon from '@local-assets/icons/deno.png';
import nodeIcon from '@local-assets/icons/node.png';
import reactIcon from '@local-assets/icons/react.png';
import solidIcon from '@local-assets/icons/solid.png';
import svelteIcon from '@local-assets/icons/svelte.png';
import vueIcon from '@local-assets/icons/vue.png';
import superSetImage from '@local-assets/superset.png';

import StackedBlock from './StackedBlock.vue';
</script>

<template>
  <section
    class="wrapper border-t grid grid-rows-3 lg:grid-rows-1 lg:grid-cols-3 lg:h-72 divide-y lg:divide-y-0 lg:divide-x"
  >
    <div class="p-5 lg:p-10 flex flex-col justify-between">
      <div class="flex flex-col gap-3 h-30 lg:h-auto">
        <h5>Runtime Agnostic</h5>
        <p>
          <span class="pr-1">Works with</span> <code>node</code> <code>bun</code> <code>deno</code>
        </p>
      </div>
      <ul class="stacked-blocks">
        <StackedBlock :src="nodeIcon" alt="node" href="https://nodejs.org" />
        <StackedBlock :src="bunIcon" alt="bun" href="https://bun.sh" />
        <StackedBlock :src="denoIcon" alt="deno" href="https://deno.com" />
      </ul>
    </div>
    <div class="p-5 lg:p-10 flex flex-col justify-between">
      <h5>Powering your favorite frameworks</h5>
      <div class="flex gap-3 items-center">
        <ul class="stacked-blocks">
          <StackedBlock :src="reactIcon" alt="react" href="https://react.dev" />
          <StackedBlock :src="vueIcon" alt="vue" href="https://vuejs.org" />
          <StackedBlock :src="svelteIcon" alt="svelte" href="https://svelte.dev" />
          <StackedBlock :src="solidIcon" alt="solid" href="https://solidjs.com" />
        </ul>
        <p class="text-base text-primary">+ 20 more</p>
      </div>
    </div>
    <div class="p-5 lg:p-10 flex flex-col gap-3 relative">
      <h5>Drop-in superset of Vite</h5>
      <p class="text-balance">Smooth, incremental adoption if you are already using Vite</p>
      <img
        loading="lazy"
        :src="superSetImage"
        alt="Vite+ is a superset of Vite"
        class="w-24 lg:w-36 absolute right-5 bottom-5"
      />
    </div>
  </section>
</template>

<style scoped></style>
