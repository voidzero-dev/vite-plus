<script setup lang="ts">
import rolldownIcon from '@assets/icons/rolldown-light.svg';
</script>

<template>
  <section id="feature-pack" class="wrapper border-t grid md:grid-cols-2 md:divide-x divide-nickel">
    <div class="px-5 py-6 md:p-10 flex flex-col justify-between gap-15">
      <div class="flex flex-col gap-5">
        <span class="text-grey text-xs font-mono uppercase tracking-wide">Vite+ pack</span>
        <h4 class="text-white">Library packaging with best practices baked in</h4>
        <p class="text-white/70 text-base max-w-[25rem] text-pretty">
          Package TS and JS libraries for npm or build standalone app binaries with a single
          <code class="mx-1 outline-none bg-nickel/50 text-aqua">vp pack</code> command.
        </p>
        <ul class="checkmark-list">
          <li>
            <code class="mx-1 outline-none bg-nickel/50 text-[#6CA1DB]">DTS</code> generation &
            bundling
          </li>
          <li>Automatic package exports generation</li>
          <li>Standalone app binaries and transform-only unbundled mode</li>
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
      <div class="bg-rolldown h-full overflow-clip flex items-center py-20 px-5 md:pl-10 md:pr-0">
        <div
          class="w-full mr-0 md:mr-10 px-5 py-6 relative bg-slate rounded-tl rounded-bl outline-1 outline-offset-[2px] outline-white/20 font-mono text-sm leading-[1.5rem] text-white"
        >
          <div class="text-white">$ vp pack</div>
          <div class="h-4" />
          <div class="text-grey">
            CLI Building entry:
            <span class="terminal-blue">src/index.ts</span>
          </div>
          <div class="text-grey">
            CLI Using config:
            <span class="text-white">tsdown.config.ts</span>
          </div>
          <div class="text-grey">
            CLI tsdown <span class="text-white">0.14.1</span> powered by Rolldown
          </div>
          <div class="h-4" />
          <div class="text-grey">
            ESM <span class="text-white">dist/index.js</span>
            <span class="inline-block w-2" aria-hidden="true"></span>
            <span class="terminal-blue">4.8 kB</span>
          </div>
          <div class="text-grey">
            DTS <span class="text-white">dist/index.d.ts</span>
            <span class="inline-block w-2" aria-hidden="true"></span>
            <span class="terminal-blue">1.2 kB</span>
          </div>
          <div class="text-grey">
            <span class="text-zest">✓</span> Pack completed in <span class="text-white">128ms</span>
          </div>
        </div>
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
