<script setup lang="ts">
import runTerminal from '@local-assets/terminal-features/run.svg';
</script>

<template>
  <section id="feature-run" class="wrapper border-t grid md:grid-cols-2 md:divide-x divide-nickel">
    <div class="px-5 py-6 md:p-10 flex flex-col justify-between gap-15">
      <div class="flex flex-col gap-5">
        <span class="text-grey text-xs font-mono uppercase tracking-wide">Vite run</span>
        <h4 class="text-white">Monorepo Caching that Just Works</h4>
        <p class="text-white/70 text-base max-w-[25rem] text-pretty">
          Built-in Turborepo / Nx alternative
        </p>
        <ul class="checkmark-list">
          <li>Task runner with caching</li>
          <li>Smart inputs inference that works even without config</li>
        </ul>
      </div>
    </div>
    <div class="flex flex-col">
      <div
        class="bg-viteplus h-full overflow-clip flex justify-end items-center py-36 px-5 md:pl-10 md:pr-0"
      >
        <img
          loading="lazy"
          :src="runTerminal"
          alt="Monorepo caching that just works"
          class="w-full md:w-auto"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.bg-viteplus {
  background-image: url('@local-assets/backgrounds/viteplus.jpg');
  background-size: cover;
  background-position: center;
}
</style>
