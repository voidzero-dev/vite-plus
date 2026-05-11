<template>
  <footer class="bg-primary" data-theme="dark">
    <Sponsors
      description="Vite+ is free and open source, made possible by a full-time team and passionate open-source contributors."
      sponsorLinkText="Contribute"
      sponsorLink="https://github.com/voidzero-dev/vite-plus/blob/main/CONTRIBUTING.md"
    />
    <section class="wrapper">
      <Footer
        heading="Take your team's productivity to the next level with Vite+"
        subheading=" "
        button-text="Get started"
        button-link="/guide"
      />
    </section>
  </footer>
</template>

<style scoped></style>
<script setup lang="ts">
import Footer from '@components/oss/Footer.vue';
import Sponsors from '@components/oss/Sponsors.vue';
</script>
