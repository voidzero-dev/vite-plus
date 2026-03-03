<script setup lang="ts">
import boltLogo from '@assets/clients/bolt.svg';
import cloudflareLogo from '@assets/clients/cloudflare.svg';
import framerLogo from '@assets/clients/framer.svg';
import huggingfaceLogo from '@assets/clients/hugging-face.svg';
import linearLogo from '@assets/clients/linear.svg';
import mercedesLogo from '@assets/clients/mercedes.svg';
import openaiLogo from '@assets/clients/openai.svg';
// Import client logos
import shopifyLogo from '@assets/clients/shopify.svg';

const logos = [
  { src: openaiLogo, alt: 'OpenAI' },
  { src: boltLogo, alt: 'Bolt' },
  { src: framerLogo, alt: 'Framer' },
  { src: linearLogo, alt: 'Linear' },
  { src: shopifyLogo, alt: 'Shopify' },
  { src: mercedesLogo, alt: 'Mercedes' },
  { src: huggingfaceLogo, alt: 'Hugging Face' },
  { src: cloudflareLogo, alt: 'Cloudflare' },
];
</script>

<template>
  <section
    class="wrapper wrapper--ticks md:pl-10 flex flex-col md:flex-row items-center border-t md:border-t-0"
  >
    <div class="w-full md:flex-1 py-6 border-b md:border-b-0 md:border-r text-center md:text-start">
      <p class="text-nickel">Trusted by the world's best software teams</p>
    </div>
    <div class="flex-1 py-6 overflow-hidden">
      <div class="scroll-container">
        <div class="scroll-content">
          <div class="flex gap-20 items-center">
            <div v-for="logo in logos" :key="logo.alt" class="logo-container">
              <img loading="lazy" :src="logo.src" :alt="logo.alt" class="grayscale" />
            </div>
          </div>
          <div class="flex gap-20 items-center">
            <div v-for="logo in logos" :key="`${logo.alt}-duplicate`" class="logo-container">
              <img loading="lazy" :src="logo.src" :alt="logo.alt" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.scroll-container {
  position: relative;
}

.scroll-content {
  display: flex;
  gap: 80px;
  animation: scroll-horizontal 20s linear infinite;
  width: fit-content;
}

.scroll-content > div {
  flex-shrink: 0;
}

.logo-container {
  width: 88px;
  height: 25px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  position: relative;
}

.scroll-content > div {
  flex-shrink: 0;
  position: relative;
}

.logo-container::after {
  content: '';
  position: absolute;
  right: -40px;
  top: -1.5rem;
  bottom: -1.5rem;
  width: 1px;
  background-color: var(--color-stroke);
}

.logo-container img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

@keyframes scroll-horizontal {
  0% {
    transform: translateX(0);
  }
  100% {
    transform: translateX(calc(-88px * 7 - 80px * 6 - 80px));
  }
}
</style>
