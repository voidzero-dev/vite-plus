<script setup lang="ts">
import uiTerminal from '@local-assets/terminal-features/ui.svg';
</script>

<template>
  <section id="feature-ui" class="wrapper border-t grid md:grid-cols-2 md:divide-x divide-nickel">
    <div class="px-5 py-6 md:p-10 flex flex-col justify-between gap-15">
      <div class="flex flex-col gap-5">
        <span class="text-grey text-xs font-mono uppercase tracking-wide">Vite UI</span>
        <h4 class="text-white">Advanced GUI Devtools</h4>
        <p class="text-white/70 text-base max-w-[25rem] text-pretty">
          Debug, visualize, & analyze: understand what’s making your build slow and what’s making
          your bundle size large
        </p>
        <ul class="checkmark-list">
          <li>Transform pipeline inspector</li>
          <li>Module dependency graph</li>
          <li>Bundle analyzer</li>
        </ul>
      </div>
    </div>
    <div class="flex flex-col">
      <div
        class="bg-viteplus h-full overflow-clip flex justify-end items-center pt-10 px-5 md:pl-10 md:pr-0"
      >
        <img
          loading="lazy"
          :src="uiTerminal"
          alt="Advanced GUI Devtools"
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
