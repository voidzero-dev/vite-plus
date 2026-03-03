<script setup lang="ts">
import nitroIcon from '@local-assets/icons/nitro.png';
import metaFrameworksImage from '@local-assets/meta-frameworks.png';
import cloudflareLogo from '@local-assets/platforms/cloudflare.svg';
import netlifyLogo from '@local-assets/platforms/netlify.svg';
import renderLogo from '@local-assets/platforms/render.svg';
import vercelLogo from '@local-assets/platforms/vercel.svg';
import nitroTerminal from '@local-assets/terminal-features/nitro.svg';
</script>

<template>
  <section
    class="wrapper wrapper--ticks border-t grid sm:grid-cols-2 divide-y sm:divide-y-0 sm:divide-x divide-stroke"
  >
    <div class="bg-beige/50 p-5 sm:p-10 flex flex-col gap-3 justify-between">
      <div class="flex flex-col gap-3">
        <h6>Meta Frameworks</h6>
        <p class="max-w-[18rem] text-balance">
          You can use meta-frameworks that ship as Vite plugins with Vite+
        </p>
      </div>
      <img loading="lazy" :src="metaFrameworksImage" alt="Meta frameworks" class="w-full mt-11" />
    </div>
    <div class="flex flex-col justify-between h-full">
      <div class="flex flex-col divide-y divide-stroke shrink-0">
        <div class="p-5 sm:p-10 flex flex-col gap-3">
          <h6>Platform Agnostic</h6>
          <p class="max-w-[18rem] text-balance">
            First-class support on Vercel, Netlify, Cloudflare & more
          </p>
        </div>
        <div
          class="grid grid-cols-2 sm:grid-cols-4 divide-y sm:divide-y-0 divide-x divide-stroke w-full flex-grow h-30 sm:h-[4.5rem]"
        >
          <!-- TODO replace with LogoGrid. Make sure mobile view is not jumpy -->
          <div class="flex items-center justify-center">
            <img
              loading="lazy"
              :src="vercelLogo"
              alt="Vercel"
              class="max-w-20 max-h-10 object-contain"
            />
          </div>
          <div class="flex items-center justify-center">
            <img
              loading="lazy"
              :src="netlifyLogo"
              alt="Netlify"
              class="max-w-20 max-h-10 object-contain"
            />
          </div>
          <div class="flex items-center justify-center">
            <img
              loading="lazy"
              :src="cloudflareLogo"
              alt="Cloudflare"
              class="max-w-20 max-h-10 object-contain"
            />
          </div>
          <div class="flex items-center justify-center">
            <img
              loading="lazy"
              :src="renderLogo"
              alt="Render"
              class="max-w-20 max-h-10 object-contain"
            />
          </div>
        </div>
      </div>
      <div class="bg-nitro bg-right-top px-10 pt-7 flex flex-col justify-end gap-5 shrink-0">
        <div class="flex flex-row items-center gap-5">
          <img loading="lazy" :src="nitroIcon" alt="Nitro icon" class="size-10 sm:size-12" />
          <h6 class="text-white drop-shadow-sm/70">Deploy anywhere by pairing with Nitro</h6>
        </div>
        <img loading="lazy" :src="nitroTerminal" alt="Deploy anywhere with Nitro" class="w-full" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.bg-nitro {
  background-image: url('@local-assets/backgrounds/nitro.jpg');
  background-size: cover;
  background-position: center;
}
</style>
