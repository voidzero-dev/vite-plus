<script setup lang="ts">
import OSSHeader from '@components/oss/Header.vue';
import BaseTheme from '@voidzero-dev/vitepress-theme/src/viteplus';
import { useData } from 'vitepress';
import { nextTick, onUnmounted, watch } from 'vue';

import Footer from './components/Footer.vue';
import Home from './layouts/Home.vue';
// import Error404 from "./layouts/Error404.vue";

const { frontmatter, isDark } = useData();
const { Layout: BaseLayout } = BaseTheme;
let homeHeaderObserver: MutationObserver | null = null;

const syncHeaderMobileMenuTheme = (header: HTMLElement | null, isHome: boolean) => {
  const mobileMenu = header?.querySelector<HTMLElement>('#mobile-menu');

  if (!mobileMenu) {
    return;
  }

  if (isHome) {
    mobileMenu.setAttribute('data-theme', 'light');
  } else {
    mobileMenu.removeAttribute('data-theme');
  }
};

const setupHomeHeaderObserver = (header: HTMLElement | null, isHome: boolean) => {
  homeHeaderObserver?.disconnect();
  homeHeaderObserver = null;

  if (!header || !isHome || typeof MutationObserver === 'undefined') {
    return;
  }

  homeHeaderObserver = new MutationObserver(() => {
    syncHeaderMobileMenuTheme(header, isHome);
  });

  homeHeaderObserver.observe(header, {
    childList: true,
    subtree: true,
  });
};

const syncHomeHeaderState = async () => {
  if (typeof document === 'undefined') {
    return;
  }

  const isHome = frontmatter.value?.layout === 'home';

  await nextTick();

  const header = document.querySelector<HTMLElement>('.home-header');

  setupHomeHeaderObserver(header, isHome);

  if (!header) {
    return;
  }

  syncHeaderMobileMenuTheme(header, isHome);
};

watch(
  [() => frontmatter.value?.layout, () => frontmatter.value?.theme, () => isDark.value],
  () => {
    void syncHomeHeaderState();
  },
  { immediate: true },
);

onUnmounted(() => {
  homeHeaderObserver?.disconnect();
  homeHeaderObserver = null;
});
</script>

<template>
  <div
    v-if="frontmatter.layout === 'home'"
    class="marketing-layout"
    :data-theme="frontmatter.theme"
  >
    <OSSHeader class="home-header" :data-theme="frontmatter.theme" />
    <Home />
    <Footer />
  </div>
  <BaseLayout v-else />
</template>
