<script setup lang="ts">
import vitestIcon from '@assets/icons/vitest-light.svg';
import testTerminal from '@local-assets/terminal-features/test.svg';
</script>
<template>
  <section id="feature-test" class="wrapper border-t grid md:grid-cols-2 divide-x divide-nickel">
    <div class="px-5 py-6 md:p-10 flex flex-col justify-between gap-15">
      <div class="flex flex-col gap-5">
        <span class="text-grey text-xs font-mono uppercase tracking-wide">Vite test</span>
        <h4 class="text-white">Testing made simple</h4>
        <p class="text-white/70 text-base max-w-[25rem] text-pretty">
          Feature rich test runner that automatically reuses the same resolve and transform config
          from your application.
        </p>
        <ul class="checkmark-list">
          <li><code class="mr-1 outline-none bg-nickel/50 text-zest">Jest</code> compatible API</li>
          <li>Test isolation by default</li>
          <li>Browser Mode: run unit tests in actual browsers</li>
          <li>Coverage reports, snapshot tests, type tests, visual regression tests...</li>
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
    <div class="flex flex-col min-h-[22rem] sm:min-h-[30rem]">
      <div class="bg-vitest pl-10 h-full flex flex-col justify-center overflow-clip">
        <div
          class="block px-5 py-6 relative bg-slate rounded-tl rounded-bl outline-1 outline-offset-[2px] outline-white/20"
        >
          <img loading="lazy" :src="testTerminal" alt="vite test terminal command" />
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
