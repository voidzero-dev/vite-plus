<template>
  <div class="wrapper md:border-none mt-10 md:mt-0">
    <RiveAnimation
      :desktop-src="homepageAnimation"
      :mobile-src="homepageAnimationMobile"
      :desktop-width="1280"
      :desktop-height="580"
      :mobile-width="253"
      :mobile-height="268"
      canvas-class="w-full"
    />
  </div>
</template>

<script setup lang="ts">
import RiveAnimation from '@components/shared/RiveAnimation.vue';
import homepageAnimationMobile from '@local-assets/animations/253_x_268_vite+_masthead_mobile.riv';
import homepageAnimation from '@local-assets/animations/1280_x_580_vite+_masthead.riv';
</script>
