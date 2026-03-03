<script setup lang="ts">
import RiveAnimation from '@components/shared/RiveAnimation.vue';
import focusOnShippingAnimation from '@local-assets/animations/514_x_246_focus_on_shipping_v2.riv';
import stayFastAtScaleAnimation from '@local-assets/animations/561_x_273_stay_fast_at_scale.riv';
import productivitySecurityImage from '@local-assets/productivity-security.png';
import tileOxc from '@local-assets/tiles/oxc.png';
import tileVite from '@local-assets/tiles/vite.png';
import tileVitest from '@local-assets/tiles/vitest.png';
</script>

<template>
  <section
    class="wrapper wrapper--ticks border-t grid lg:grid-cols-2 divide-x divide-y divide-stroke"
  >
    <div class="p-5 sm:p-10 flex flex-col gap-3">
      <h5 class="text-balance sm:text-pretty">A trusted stack to standardize on</h5>
      <p class="sm:max-w-[28rem] text-pretty mb-12 sm:mb-16">
        Vite+ is built on established open source industry standards, and maintained by the same
        experts behind these projects.
      </p>
      <div class="flex flex-col gap-6 sm:pr-10 -mx-5 sm:mx-0 divide-y sm:divide-y-0 divide-stroke">
        <div class="flex flex-col gap-6 pt-6 pb-6 sm:pb-0 px-5 sm:px-0 sm:pt-0 first:pt-0">
          <img loading="lazy" :src="tileVite" alt="Vite" class="w-24 flex-shrink-0 sm:hidden" />
          <div class="flex flex-row sm:items-center gap-6">
            <img
              loading="lazy"
              :src="tileVite"
              alt="Vite"
              class="w-24 flex-shrink-0 hidden sm:block"
            />
            <div
              class="relative flex flex-col flex-1 sm:pl-6 before:content-none sm:before:content-[''] before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:h-full before:w-px before:bg-stroke"
            >
              <p class="text-primary font-medium text-base sm:text-xl">36m+</p>
              <p class="leading-tight text-base">Weekly npm downloads</p>
            </div>
            <div
              class="relative flex flex-col flex-shrink-0 pl-6 before:content-[''] before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:h-full before:w-px before:bg-stroke"
            >
              <p class="text-primary font-medium text-base sm:text-xl">75k</p>
              <p class="leading-tight text-base">GitHub stars</p>
            </div>
          </div>
        </div>
        <div class="flex flex-col gap-6 pt-6 pb-6 sm:pb-0 px-5 sm:px-0 sm:pt-0 first:pt-0">
          <img loading="lazy" :src="tileVitest" alt="Vitest" class="w-24 flex-shrink-0 sm:hidden" />
          <div class="flex flex-row sm:items-center gap-6">
            <img
              loading="lazy"
              :src="tileVitest"
              alt="Vitest"
              class="w-24 flex-shrink-0 hidden sm:block"
            />
            <div
              class="relative flex flex-col flex-1 sm:pl-6 before:content-none sm:before:content-[''] before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:h-full before:w-px before:bg-stroke"
            >
              <p class="text-primary font-medium text-base sm:text-xl">16.5m+</p>
              <p class="leading-tight text-base">Weekly npm downloads</p>
            </div>
            <div
              class="relative flex flex-col flex-shrink-0 pl-6 before:content-[''] before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:h-full before:w-px before:bg-stroke"
            >
              <p class="text-primary font-medium text-base sm:text-xl">15k</p>
              <p class="leading-tight text-base">GitHub stars</p>
            </div>
          </div>
        </div>
        <div class="flex flex-col gap-6 pt-6 pb-6 sm:pb-0 px-5 sm:px-0 sm:pt-0 first:pt-0">
          <img loading="lazy" :src="tileOxc" alt="Oxc" class="w-24 flex-shrink-0 sm:hidden" />
          <div class="flex flex-row sm:items-center gap-6">
            <img
              loading="lazy"
              :src="tileOxc"
              alt="Oxc"
              class="w-24 flex-shrink-0 hidden sm:block"
            />
            <div
              class="relative flex flex-col flex-1 sm:pl-6 before:content-none sm:before:content-[''] before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:h-full before:w-px before:bg-stroke"
            >
              <p class="text-primary font-medium text-base sm:text-xl">3.8m+</p>
              <p class="leading-tight text-base">Weekly npm downloads</p>
            </div>
            <div
              class="relative flex flex-col flex-shrink-0 pl-6 before:content-[''] before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:h-full before:w-px before:bg-stroke"
            >
              <p class="text-primary font-medium text-base sm:text-xl">16.5k</p>
              <p class="leading-tight text-base">GitHub stars</p>
            </div>
          </div>
        </div>
      </div>
    </div>
    <div class="p-5 sm:p-10 flex flex-col gap-3 border-r-0">
      <h5>Stay fast at scale</h5>
      <p class="max-w-[26rem] text-pretty">
        With low-level components written in Rust, Vite+ offers unparalleled performance at
        enterprise scale: up to <span class="text-primary">40x faster builds</span> than webpack and
        <span class="text-primary">100x faster linting</span> than ESLint.
      </p>
      <RiveAnimation
        :desktop-src="stayFastAtScaleAnimation"
        :desktop-width="561"
        :desktop-height="273"
        canvas-class="w-full mt-10"
      />
    </div>
    <div class="p-5 sm:p-10 flex flex-col gap-3">
      <h5>Focus on shipping, not tooling</h5>
      <ul class="list-disc pl-3 flex flex-col gap-2 marker:text-ruby mt-1">
        <li class="leading-tight">Stop wasting time on tooling maintenance</li>
        <li class="leading-tight">Improve cross-team developer mobility</li>
        <li class="leading-tight">
          Standardize and enforce best practices, especially for AI-assisted workflows
        </li>
      </ul>
      <RiveAnimation
        :desktop-src="focusOnShippingAnimation"
        :desktop-width="514"
        :desktop-height="246"
        canvas-class="w-full mt-10"
      />
    </div>
    <div class="p-5 sm:p-10 flex flex-col gap-3">
      <h5>Supply chain security</h5>
      <p class="max-w-[20rem] text-pretty">
        Vite+ development undergoes the most rigorous security practices and we vet all its
        dependencies. SLAs available.
      </p>
      <img
        loading="lazy"
        class="w-full"
        :src="productivitySecurityImage"
        alt="Vite+ vets all dependencies with rigorous security practices"
      />
    </div>
  </section>
</template>

<style scoped></style>
