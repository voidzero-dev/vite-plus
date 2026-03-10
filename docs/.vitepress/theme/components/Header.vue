<script setup lang="ts">
import { Icon } from '@iconify/vue';
import { onMounted, onUnmounted, ref } from 'vue';

const nav = [{ text: 'Docs', link: '/guide' }];

const mobileMenuOpen = ref(false);
const expandedMobileItem = ref<string | null>(null);

const lockBodyScroll = () => {
  const scrollbarWidth = window.innerWidth - document.documentElement.clientWidth;
  document.body.style.overflow = 'hidden';
  document.body.style.position = 'fixed';
  document.body.style.width = '100%';
  document.body.style.top = '0';
  if (scrollbarWidth > 0) {
    document.body.style.paddingRight = `${scrollbarWidth}px`;
  }
};

const unlockBodyScroll = () => {
  document.body.style.overflow = '';
  document.body.style.position = '';
  document.body.style.width = '';
  document.body.style.top = '';
  document.body.style.paddingRight = '';
};

const closeMobileMenu = () => {
  mobileMenuOpen.value = false;
  expandedMobileItem.value = null;
  unlockBodyScroll();
};

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && mobileMenuOpen.value) {
    closeMobileMenu();
  }
};

const toggleMobileMenu = () => {
  mobileMenuOpen.value = !mobileMenuOpen.value;
  if (mobileMenuOpen.value) {
    lockBodyScroll();
    expandedMobileItem.value = null;
  } else {
    unlockBodyScroll();
    expandedMobileItem.value = null;
  }
};

onMounted(() => {
  document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown);
  unlockBodyScroll();
});
</script>

<template>
  <header class="wrapper px-6 py-5 flex items-center justify-between relative">
    <div class="flex gap-10 self-stretch">
      <a href="/" class="flex flex-col items-start justify-center -mx-2 px-2">
        <img class="h-4" src="/logo.svg" alt="Vite+" />
      </a>

      <nav class="nav-container hidden lg:flex items-center gap-4">
        <ul class="nav flex items-center gap-1">
          <li v-for="navItem in nav" :key="navItem.link">
            <a
              :href="navItem.link"
              :target="navItem.link?.startsWith('http') ? '_blank' : '_self'"
              :rel="navItem.link?.startsWith('http') ? 'noopener noreferrer' : undefined"
              class="flex items-center gap-1 px-3 py-2 text-base font-heading text-primary hover:opacity-85 transition-opacity whitespace-nowrap"
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
      <a href="/guide" target="_self" class="button hidden lg:block"> Get started </a>

      <button
        @click="toggleMobileMenu"
        :aria-expanded="mobileMenuOpen"
        aria-controls="mobile-menu"
        aria-label="Toggle navigation menu"
        class="lg:hidden p-2 -mr-2 text-primary dark:text-white hover:opacity-70 transition-opacity cursor-pointer"
        type="button"
      >
        <svg
          v-if="!mobileMenuOpen"
          class="size-6 block dark:hidden"
          viewBox="0 0 18 8"
          xmlns="http://www.w3.org/2000/svg"
        >
          <path d="M0 0.75H18" stroke="#08060D" stroke-width="1.5" />
          <path d="M0 6.75H18" stroke="#08060D" stroke-width="1.5" />
        </svg>
        <svg
          v-if="!mobileMenuOpen"
          class="size-6 hidden dark:block"
          viewBox="0 0 18 8"
          xmlns="http://www.w3.org/2000/svg"
        >
          <path d="M0 0.75H18" stroke="#FFFFFF" stroke-width="1.5" />
          <path d="M0 6.75H18" stroke="#FFFFFF" stroke-width="1.5" />
        </svg>
      </button>
    </div>
  </header>

  <div
    v-if="mobileMenuOpen"
    id="mobile-menu"
    role="dialog"
    aria-modal="true"
    aria-label="Mobile navigation menu"
    data-theme="dark"
    class="lg:hidden fixed inset-0 z-[1001] bg-primary"
  >
    <section class="wrapper animate-fade-in">
      <div class="w-full pl-5 pr-7 py-5 lg:py-7 flex items-center justify-between">
        <a href="/">
          <img class="h-4" src="@assets/logos/viteplus-light.svg" alt="Vite+" />
        </a>
        <button
          @click="closeMobileMenu"
          aria-label="Close navigation menu"
          class="p-2 -mr-2 text-white hover:opacity-70 transition-opacity"
          type="button"
        >
          <svg
            class="size-6 cursor-pointer"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
      </div>

      <div
        class="overflow-y-auto flex flex-col [scrollbar-width:none] [-ms-overflow-style:none] [&::-webkit-scrollbar]:hidden"
        style="height: calc(100vh - 88px)"
      >
        <nav class="flex-1 w-full pt-6 pb-8">
          <ul class="space-y-1">
            <li v-for="navItem in nav" :key="navItem.link">
              <a
                :href="navItem.link"
                @click="closeMobileMenu"
                :target="navItem.link?.startsWith('http') ? '_blank' : '_self'"
                :rel="navItem.link?.startsWith('http') ? 'noopener noreferrer' : undefined"
                class="flex py-3 px-4 text-lg font-sans text-white items-center justify-between"
              >
                {{ navItem.text }}
                <svg
                  v-if="navItem.link?.startsWith('http')"
                  class="inline-block ml-1 size-4"
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

        <div class="w-full py-12 border-t border-nickel relative tick-left tick-right mt-auto">
          <div class="space-y-12">
            <div class="px-6">
              <a
                href="/guide"
                target="_self"
                class="button button--primary button--white block text-center bg-white text-primary hover:bg-white/90"
                @click="closeMobileMenu"
              >
                <span>Get started</span>
              </a>
            </div>

            <div class="border-t border-nickel tick-left tick-right relative"></div>

            <div class="flex items-center justify-center gap-4 pb-12">
              <a
                href="https://github.com/voidzero-dev"
                target="_blank"
                rel="noopener noreferrer"
                class="hover:opacity-70 transition-opacity"
                @click="closeMobileMenu"
              >
                <Icon icon="simple-icons:github" aria-label="GitHub" class="size-5 text-white" />
              </a>
              <a
                href="https://x.com/voidzerodev"
                target="_blank"
                rel="noopener noreferrer"
                class="hover:opacity-70 transition-opacity"
                @click="closeMobileMenu"
              >
                <Icon icon="simple-icons:x" aria-label="X" class="size-5 text-white" />
              </a>
              <a
                href="https://discord.gg/cC6TEVFKSx"
                target="_blank"
                rel="noopener noreferrer"
                class="hover:opacity-70 transition-opacity"
                @click="closeMobileMenu"
              >
                <Icon icon="simple-icons:discord" aria-label="Discord" class="size-5 text-white" />
              </a>
              <a
                href="https://bsky.app/profile/voidzero.dev"
                target="_blank"
                rel="noopener noreferrer"
                class="hover:opacity-70 transition-opacity"
                @click="closeMobileMenu"
              >
                <Icon icon="simple-icons:bluesky" aria-label="Bluesky" class="size-5 text-white" />
              </a>
            </div>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
@keyframes shadowFadeIn {
  from {
    box-shadow:
      rgba(50, 50, 93, 0.1) 0px 25px 50px -20px,
      rgba(0, 0, 0, 0.15) 0px 15px 30px -30px;
  }

  to {
    box-shadow:
      rgba(50, 50, 93, 0.25) 0px 50px 100px -20px,
      rgba(0, 0, 0, 0.3) 0px 30px 60px -30px;
  }
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }

  to {
    opacity: 1;
  }
}
</style>
