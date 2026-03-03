<script setup lang="ts">
import rolldownIcon from '@assets/icons/rolldown-light.svg';
import viteIcon from '@assets/icons/vite-light.svg';
import devTerminal from '@local-assets/terminal-features/dev.svg';
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
          Spin up dev servers and create production builds with extreme speed. Stay in the flow and
          keep CI fast.
        </p>
        <ul class="checkmark-list">
          <li>
            Always instant
            <code class="mr-1 outline-none bg-nickel/50 text-vite"
              >Hot Module Replacement (HMR)</code
            >
          </li>
          <li>40x faster production build than webpack</li>
          <li>Opt-in full-bundle dev mode for large apps</li>
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
    <div class="flex flex-col min-h-[22rem] sm:min-h-[30rem]">
      <div class="bg-vite pl-10 h-full flex flex-col justify-center overflow-clip">
        <div
          class="block px-5 py-6 relative bg-slate rounded-tl rounded-bl outline-1 outline-offset-[2px] outline-white/20"
        >
          <img loading="lazy" :src="devTerminal" alt="vp build terminal command" />
        </div>
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
