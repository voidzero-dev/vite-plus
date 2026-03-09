<template>
  <footer class="bg-primary" data-theme="dark">
    <Sponsors
      description="Vite+ is free and open source, made possible by a full-time team and passionate open-source contributors."
      sponsorLinkText="Contribute"
      sponsorLink="https://github.com/voidzero-dev/vite-plus/blob/main/CONTRIBUTING.md"
    />
    <section class="wrapper">
      <div
        class="bg-wine bg-[url(/cta-background.jpg)] bg-cover py-16 md:py-30 px-5 md:px-0 overflow-clip flex flex-col items-center justify-center gap-8 md:gap-12"
      >
        <h2 class="text-white w-full md:w-2xl text-center text-balance">
          Take your team's productivity to the next level with Vite+
        </h2>
        <div class="flex items-center gap-5">
          <a href="/vite/guide" target="_self" class="button button--white">Get started</a>
          <!-- <a
            href="https://voidzero.dev/posts/announcing-vite-plus?utm_source=viteplusdev&utm_content=bottom_learn_more"
            target="_blank"
            rel="noopener noreferrer"
            class="button button--white"
          >
            Learn more
          </a> -->
        </div>
      </div>
      <div
        class="px-5 md:px-24 pt-10 md:pt-16 pb-16 md:pb-30 flex flex-col md:flex-row gap-10 md:gap-0 md:justify-between"
      >
        <div>
          <p class="text-grey text-xs font-mono uppercase tracking-wide mb-8">Company</p>
          <ul class="flex flex-col gap-3">
            <li>
              <a
                href="https://voidzero.dev/"
                target="_blank"
                rel="noopener noreferrer"
                class="text-white text-base"
                >VoidZero</a
              >
            </li>
            <li>
              <a
                href="https://vite.dev/"
                target="_blank"
                rel="noopener noreferrer"
                class="text-white text-base"
                >Vite</a
              >
            </li>
            <li>
              <a
                href="https://vitest.dev/"
                target="_blank"
                rel="noopener noreferrer"
                class="text-white text-base"
                >Vitest</a
              >
            </li>
            <li>
              <a
                href="https://rolldown.rs/"
                target="_blank"
                rel="noopener noreferrer"
                class="text-white text-base"
                >Rolldown</a
              >
            </li>

            <li>
              <a
                href="https://oxc.rs/"
                target="_blank"
                rel="noopener noreferrer"
                class="text-white text-base"
                >Oxc</a
              >
            </li>
          </ul>
        </div>
        <div>
          <p class="text-grey text-xs font-mono uppercase tracking-wide mb-8">Social</p>
          <ul class="flex flex-col gap-3">
            <li>
              <a
                href="https://github.com/voidzero-dev"
                target="_blank"
                rel="noopener noreferrer"
                class="text-white text-base flex gap-3 items-center"
                ><Icon icon="simple-icons:github" aria-label="GitHub" class="size-[18px]" />
                GitHub</a
              >
            </li>
            <li>
              <a
                href="https://x.com/voidzerodev"
                target="_blank"
                rel="noopener noreferrer"
                class="text-white text-base flex gap-3 items-center"
                ><Icon icon="simple-icons:x" aria-label="X" class="size-[18px]" />X.com</a
              >
            </li>
            <li>
              <a
                href="https://bsky.app/profile/voidzero.dev"
                target="_blank"
                rel="noopener noreferrer"
                class="text-white text-base flex gap-3 items-center"
                ><Icon icon="simple-icons:bluesky" aria-label="Bluesky" class="size-[18px]" />
                Bluesky</a
              >
            </li>
          </ul>
        </div>
      </div>
    </section>
    <section class="wrapper wrapper--ticks border-t py-5 px-5 md:px-24">
      <p class="text-sm">
        © {{ new Date().getFullYear() }} VoidZero Inc.
        <span class="hidden sm:inline">All Rights Reserved.</span>
      </p>
    </section>
  </footer>
</template>

<style scoped></style>
<script setup lang="ts">
import Sponsors from '@components/oss/Sponsors.vue';
import { Icon } from '@iconify/vue';
</script>
