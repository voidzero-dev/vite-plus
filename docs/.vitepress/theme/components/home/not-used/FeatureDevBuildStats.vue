<script setup lang="ts">
import rolldownIcon from '@assets/icons/rolldown-light.svg';
import viteIcon from '@assets/icons/vite-light.svg';
import viteBackground from '@local-assets/backgrounds/vite.jpg';
import devTerminal from '@local-assets/terminal-features/dev.svg';
import { TabsContent, TabsList, TabsRoot, TabsTrigger } from 'reka-ui';

import { devPerformance, buildPerformance } from '../../../data/performance';
import PerformanceBar from './PerformanceBar.vue';
</script>

<template>
  <section
    id="feature-dev-build"
    class="wrapper border-t grid md:grid-cols-2 divide-x divide-nickel"
  >
    <div class="px-5 py-6 md:p-10 flex flex-col justify-between gap-15">
      <div class="flex flex-col gap-5">
        <span class="text-grey text-xs font-mono uppercase tracking-wide"
          >Vite+ dev &amp; build</span
        >
        <h4 class="text-white">Blazingly fast builds</h4>
        <p class="text-white/70 text-base max-w-[25rem] text-pretty">
          Spin up dev servers and create production builds quickly and simply.
        </p>
        <ul class="checkmark-list">
          <li>Instant feedback that never slows you down</li>
          <li>Huge ecosystem of plugins</li>
        </ul>
      </div>
      <div class="px-3 py-1.5 bg-slate rounded w-fit flex gap-2 items-center">
        <span class="text-grey text-sm font-mono hidden md:inline">Powered by</span>
        <a href="https://vite.dev/" target="_blank">
          <figure class="project-icon">
            <img loading="lazy" :src="viteIcon" alt="Vite" class="w-[20px] h-[12px]" />
            <figcaption>Vite</figcaption>
          </figure>
        </a>
        <span class="text-grey text-sm font-mono">&amp;</span>
        <a href="https://rolldown.rs/" target="_blank">
          <figure class="project-icon">
            <img loading="lazy" :src="rolldownIcon" alt="Rolldown" class="w-[20px] h-[12px]" />
            <figcaption>Rolldown</figcaption>
          </figure>
        </a>
      </div>
    </div>
    <div class="flex flex-col">
      <div class="bg-vite pl-10 pt-10 overflow-clip">
        <div
          class="block px-5 py-6 relative bg-slate rounded-tl outline-1 outline-offset-[2px] outline-white/20"
        >
          <img loading="lazy" :src="devTerminal" alt="vp build terminal command" />
        </div>
      </div>
      <div class="p-10">
        <TabsRoot default-value="tab1">
          <div class="flex flex-col md:flex-row gap-3 md:items-center mb-12 md:mb-20">
            <span class="text-grey text-xs font-mono uppercase tracking-wide block md:w-36"
              >Performance</span
            >
            <TabsList
              aria-label="features"
              class="flex items-center p-0.5 rounded-md bg-nickel/20 w-fit"
            >
              <TabsTrigger value="tab1" class="text-xs"> dev </TabsTrigger>
              <TabsTrigger value="tab2" class="text-xs"> build </TabsTrigger>
            </TabsList>
          </div>
          <TabsContent value="tab1">
            <div class="flex flex-col gap-4">
              <PerformanceBar
                v-for="item in devPerformance"
                :key="item.name"
                :data="item"
                :background-image="viteBackground"
              />
            </div>
          </TabsContent>
          <TabsContent value="tab2">
            <div class="flex flex-col gap-4">
              <PerformanceBar
                v-for="item in buildPerformance"
                :key="item.name"
                :data="item"
                :background-image="viteBackground"
              />
            </div>
          </TabsContent>
        </TabsRoot>
      </div>
    </div>
  </section>
</template>

<style scoped>
.bg-vite {
  background-image: url('@local-assets/backgrounds/vite.jpg');
  background-size: cover;
  background-position: center;
}
</style>
