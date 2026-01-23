<script setup lang="ts">
import oxcIcon from '@assets/icons/oxc-light.svg';
import lintTerminal from '@local-assets/terminal-features/lint.svg';
</script>
<template>
  <section id="feature-lint" class="wrapper border-t grid md:grid-cols-2 divide-x divide-nickel">
    <div class="px-5 py-6 md:p-10 flex flex-col justify-between gap-15">
      <div class="flex flex-col gap-5">
        <span class="text-grey text-xs font-mono uppercase tracking-wide">Vite lint</span>
        <h4 class="text-white">Catch bugs before they make it to production</h4>
        <p class="text-white/70 text-base max-w-[25rem] text-pretty">
          Analyze JavaScript code to find and fix problems
        </p>
        <ul class="checkmark-list">
          <li>
            600+ <code class="mx-1 outline-none bg-nickel/50 text-aqua">EsLint</code> compatible
            rules
          </li>
          <li>Up to 100x faster than ESLint</li>
          <li>Support for ESLint custom rules written in JS</li>
          <li>Type-aware linting</li>
        </ul>
      </div>
      <div class="px-3 py-1.5 bg-slate rounded w-fit flex gap-2 items-center">
        <span class="text-grey text-sm font-mono hidden md:inline">Powered by</span>
        <a href="https://oxc.rs/" target="_blank">
          <figure class="project-icon">
            <img loading="lazy" :src="oxcIcon" alt="Oxc" class="w-[20px] h-[12px]" />
            <figcaption>Oxc / Oxlint</figcaption>
          </figure>
        </a>
      </div>
    </div>
    <div class="flex flex-col min-h-[22rem] sm:min-h-[30rem]">
      <div class="bg-oxc pl-10 h-full flex flex-col justify-center overflow-clip">
        <div
          class="block pl-5 py-6 relative bg-slate rounded-tl rounded-bl outline-1 outline-offset-[2px] outline-white/20"
        >
          <img loading="lazy" :src="lintTerminal" alt="vite lint terminal command" />
        </div>
      </div>
    </div>
  </section>
</template>
<style scoped>
.bg-oxc {
  background-image: url('@local-assets/backgrounds/oxc.jpg');
  background-size: cover;
  background-position: center;
}
</style>
