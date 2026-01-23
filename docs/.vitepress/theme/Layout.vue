<script setup lang="ts">
import { useData } from 'vitepress';
import BaseTheme from '@voidzero-dev/vitepress-theme/src/viteplus';
import Header from './components/Header.vue';
import Footer from './components/Footer.vue';
import Home from './layouts/Home.vue';
// import Error404 from "./layouts/Error404.vue";

const { frontmatter } = useData();
const { Layout: BaseLayout } = BaseTheme;
</script>

<template>
  <div v-if="frontmatter.layout === 'home'" class="marketing-layout">
    <Header />
    <Home />
    <Footer />
  </div>
  <BaseLayout v-else />
</template>
