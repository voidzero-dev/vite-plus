<script setup lang="ts">
import vitestIcon from '@assets/icons/vitest-light.svg';
import vitestBackground from '@local-assets/backgrounds/vitest.jpg';
import testTerminal from '@local-assets/terminal-features/test.svg';

import { testPerformance } from '../../../data/performance';
import PerformanceBar from './PerformanceBar.vue';
</script>

<template>
  <section id="feature-test" class="wrapper border-t grid md:grid-cols-2 divide-x divide-nickel">
    <div class="px-5 py-6 md:p-10 flex flex-col justify-between gap-15">
      <div class="flex flex-col gap-5">
        <span class="text-grey text-xs font-mono uppercase tracking-wide">Vite+ test</span>
        <h4 class="text-white">Run tests without duplicate configurations</h4>
        <p class="text-white/70 text-base max-w-[25rem] text-pretty">
          A test runner that uses the same app configurations and shares a common transformation
          pipeline during dev, build, and test time.
        </p>
        <ul class="checkmark-list">
          <li><code class="mr-1 outline-none bg-nickel/50 text-zest">Jest</code> compatible</li>
          <li>Run your tests in actual browsers</li>
          <li>Test sharding, type, visual regression</li>
        </ul>
      </div>
      <div class="px-3 py-1.5 bg-slate rounded w-fit flex gap-2 items-center">
        <span class="text-grey text-sm font-mono hidden md:inline">Powered by</span>
        <a href="https://vitest.dev/" target="_blank">
          <figure class="project-icon">
            <img loading="lazy" :src="vitestIcon" alt="Vitest" class="w-[20px] h-[12px]" />
            <figcaption>Vitest</figcaption>
          </figure>
        </a>
      </div>
    </div>
    <div class="flex flex-col">
      <div class="bg-vitest pl-10 pt-10 overflow-clip">
        <div
          class="block px-5 py-6 relative bg-slate rounded-tl outline-1 outline-offset-[2px] outline-white/20"
        >
          <img loading="lazy" :src="testTerminal" alt="vp build terminal command" />
        </div>
      </div>
      <div class="p-10">
        <span
          class="text-grey text-xs font-mono uppercase tracking-wide block w-24 md:w-36 mb-12 md:mb-20"
          >Performance</span
        >
        <div class="flex flex-col gap-4">
          <PerformanceBar
            v-for="item in testPerformance"
            :key="item.name"
            :data="item"
            :background-image="vitestBackground"
          />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.bg-vitest {
  background-image: url('@local-assets/backgrounds/vitest.jpg');
  background-size: cover;
  background-position: center;
}
</style>
