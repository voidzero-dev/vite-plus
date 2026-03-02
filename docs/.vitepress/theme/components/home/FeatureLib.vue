<script setup lang="ts">
import rolldownIcon from '@assets/icons/rolldown-light.svg';
import libTerminal from '@local-assets/terminal-features/lib.svg';
</script>

<template>
  <section id="feature-lib" class="wrapper border-t grid md:grid-cols-2 md:divide-x divide-nickel">
    <div class="px-5 py-6 md:p-10 flex flex-col justify-between gap-15">
      <div class="flex flex-col gap-5">
        <span class="text-grey text-xs font-mono uppercase tracking-wide">Vite lib</span>
        <h4 class="text-white">Library bundling with best-practices baked in</h4>
        <p class="text-white/70 text-base max-w-[25rem] text-pretty">
          An efficient way to bundle TS and JS libraries
        </p>
        <ul class="checkmark-list">
          <li>
            <code class="mx-1 outline-none bg-nickel/50 text-[#6CA1DB]">DTS</code> generation &
            bundling
          </li>
          <li>Automatic package exports generation</li>
          <li>Transform-only unbundled mode</li>
        </ul>
      </div>
      <div class="px-3 py-1.5 bg-slate rounded w-fit flex gap-2 items-center">
        <span class="text-grey text-sm font-mono hidden md:inline">Powered by</span>
        <a href="https://rolldown.rs/" target="_blank">
          <figure class="project-icon">
            <img loading="lazy" :src="rolldownIcon" alt="Rolldown" class="w-[20px] h-[12px]" />
            <figcaption>Rolldown</figcaption>
          </figure>
        </a>
        <span>/</span>
        <a href="https://tsdown.dev/" target="_blank"> tsdown </a>
      </div>
    </div>
    <div class="flex flex-col">
      <div
        class="bg-rolldown h-full overflow-clip flex justify-end items-center py-20 px-5 md:pl-10 md:pr-0"
      >
        <img
          loading="lazy"
          :src="libTerminal"
          alt="Library bundling with best-practices baked in"
          class="w-full md:w-auto"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.bg-rolldown {
  background-image: url('@local-assets/backgrounds/rolldown.jpg');
  background-size: cover;
  background-position: center;
}
</style>
