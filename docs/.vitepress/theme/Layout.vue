<script setup lang="ts">
import BaseTheme from '@voidzero-dev/vitepress-theme/src/viteplus';
import { useData } from 'vitepress';
import { onMounted, watch } from 'vue';

import Footer from './components/Footer.vue';
import Header from './components/Header.vue';
import Home from './layouts/Home.vue';
// import Error404 from "./layouts/Error404.vue";

const { frontmatter } = useData();
const { Layout: BaseLayout } = BaseTheme;

const ensureHomeLight = () => {
  if (frontmatter.value?.layout !== 'home' || typeof document === 'undefined') {
    return;
  }

  document.documentElement.classList.remove('dark');
};

onMounted(() => {
  ensureHomeLight();
});

watch(
  () => frontmatter.value?.layout,
  () => {
    ensureHomeLight();
  },
);
</script>

<template>
  <div v-if="frontmatter.layout === 'home'" class="marketing-layout">
    <Header />
    <Home />
    <Footer />
  </div>
  <BaseLayout v-else />
</template>
