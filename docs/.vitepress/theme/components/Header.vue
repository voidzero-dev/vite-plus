<script setup lang="ts">
import { useData, useRoute } from 'vitepress';

const nav = [{ text: 'Docs', link: '/vite/guide' }];
</script>

<template>
  <header class="wrapper px-6 py-7 flex items-center justify-between">
    <div class="flex items-center gap-10">
      <a href="/">
        <img class="h-4" src="/logo.svg" alt="Vite+" />
      </a>

      <!-- Desktop navigation - hidden on mobile -->
      <nav class="nav-container hidden md:block">
        <ul class="nav">
          <li v-for="navItem in nav" :key="navItem.link" class="inline-block px-5">
            <a
              :href="navItem.link"
              :target="navItem.link?.startsWith('http') ? '_blank' : '_self'"
              :rel="navItem.link?.startsWith('http') ? 'noopener noreferrer' : undefined"
            >
              {{ navItem.text }}
              <svg
                v-if="navItem.link?.startsWith('http')"
                class="inline-block ml-1 size-3"
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 12 12"
                fill="none"
                aria-hidden="true"
              >
                <path
                  d="M2.81802 2.81803L9.18198 2.81803L9.18198 9.18199"
                  class="stroke-primary dark:stroke-white"
                  stroke-width="1.5"
                />
                <path
                  d="M9.18213 2.81802L2.81817 9.18198"
                  class="stroke-primary dark:stroke-white"
                  stroke-width="1.5"
                />
              </svg>
            </a>
          </li>
        </ul>
      </nav>
    </div>

    <div class="flex items-center gap-6">
      <span class="flex items-center gap-1 text-grey text-sm">
        By
        <a href="https://voidzero.dev/" target="_blank" rel="noopener noreferrer">
          <img class="h-3" src="@assets/logos/voidzero-dark.svg" alt="VoidZero" />
        </a>
      </span>
      <a
        href="https://tally.so/r/nGWebL"
        target="_blank"
        rel="noopener noreferrer"
        class="button hidden sm:block"
      >
        Join early access
      </a>
    </div>
  </header>
</template>

<style scoped></style>
