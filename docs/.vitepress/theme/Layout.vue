<script setup lang="ts">
import OSSHeader from '@components/oss/Header.vue';
import BaseTheme from '@voidzero-dev/vitepress-theme/src/viteplus';
import { useData } from 'vitepress';
import { nextTick, watch } from 'vue';

import Footer from './components/Footer.vue';
import Home from './layouts/Home.vue';
// import Error404 from "./layouts/Error404.vue";

const { frontmatter, isDark } = useData();
const { Layout: BaseLayout } = BaseTheme;

const syncHomeThemeOverride = async () => {
  if (typeof document === 'undefined') {
    return;
  }

  const isHome = frontmatter.value?.layout === 'home';
  const root = document.documentElement;

  if (isHome) {
    root.setAttribute('data-theme', 'light');
  } else {
    root.removeAttribute('data-theme');
  }

  await nextTick();

  const header = document.querySelector<HTMLElement>('.home-header');

  if (!header) {
    return;
  }

  if (isHome) {
    header.setAttribute('data-theme', 'light');
  } else {
    header.removeAttribute('data-theme');
  }
};

watch(
  [() => frontmatter.value?.layout, () => isDark.value],
  () => {
    void syncHomeThemeOverride();
  },
  { immediate: true },
);
</script>

<template>
  <div v-if="frontmatter.layout === 'home'" class="marketing-layout">
    <OSSHeader class="home-header" />
    <Home />
    <Footer />
  </div>
  <BaseLayout v-else />
</template>
