<script setup lang="ts">
import { TabsList, TabsRoot, TabsTrigger } from 'reka-ui';
import { computed, onMounted, onUnmounted, ref } from 'vue';

import { terminalTranscripts } from '../../data/terminal-transcripts';
import TerminalTranscript from './TerminalTranscript.vue';

// Auto-progression configuration
const AUTO_ADVANCE_DELAY = 1500;

// State management
const activeTab = ref(terminalTranscripts[0].id);
const autoPlayEnabled = ref(true);
let autoAdvanceTimeout: ReturnType<typeof setTimeout> | null = null;

// Intersection Observer state
const sectionRef = ref<HTMLElement | null>(null);
const isVisible = ref(false);
let observer: IntersectionObserver | null = null;

// Tab progression logic
const tabSequence = terminalTranscripts.map((transcript) => transcript.id);

const activeTranscript = computed(
  () =>
    terminalTranscripts.find((transcript) => transcript.id === activeTab.value) ??
    terminalTranscripts[0],
);

const goToNextTab = () => {
  const currentIndex = tabSequence.indexOf(activeTab.value);
  const nextIndex = (currentIndex + 1) % tabSequence.length;
  activeTab.value = tabSequence[nextIndex];
};

// Handle animation completion
const onAnimationComplete = () => {
  if (!autoPlayEnabled.value) {
    return;
  }

  // Clear any existing timeout
  if (autoAdvanceTimeout) {
    clearTimeout(autoAdvanceTimeout);
  }

  // Schedule next tab
  autoAdvanceTimeout = setTimeout(() => {
    goToNextTab();
  }, AUTO_ADVANCE_DELAY);
};

// Handle user interaction with tabs
const onTabChange = () => {
  // User clicked a tab, disable auto-play
  autoPlayEnabled.value = false;

  // Clear any pending auto-advance
  if (autoAdvanceTimeout) {
    clearTimeout(autoAdvanceTimeout);
    autoAdvanceTimeout = null;
  }
};

// Setup Intersection Observer
onMounted(() => {
  if (!sectionRef.value) {
    return;
  }

  observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting && !isVisible.value) {
          isVisible.value = true;
          // Disconnect observer after first intersection
          observer?.disconnect();
        }
      });
    },
    {
      threshold: 0.2, // Trigger when 20% of the element is visible
      rootMargin: '0px',
    },
  );

  observer.observe(sectionRef.value);
});

// Cleanup
onUnmounted(() => {
  if (autoAdvanceTimeout) {
    clearTimeout(autoAdvanceTimeout);
  }
  if (observer) {
    observer.disconnect();
  }
});
</script>

<template>
  <section
    ref="sectionRef"
    class="wrapper border-t h-[40rem] bg-wine terminal-background bg-cover bg-top flex justify-center pt-28 overflow-clip"
  >
    <div
      :class="[
        'self-stretch px-4 sm:px-8 py-5 sm:py-7 relative bg-[#111] rounded-tl-lg rounded-tr-lg inline-flex flex-col justify-start items-start gap-2 overflow-hidden w-[62rem] outline-1 outline-offset-[3px] outline-white/30',
        'transition-transform duration-1000',
        isVisible ? 'translate-y-0' : 'translate-y-24',
      ]"
      style="transition-timing-function: cubic-bezier(0.16, 1, 0.3, 1)"
    >
      <TabsRoot v-if="isVisible" v-model="activeTab" @update:modelValue="onTabChange">
        <div class="w-full">
          <TerminalTranscript
            :key="activeTranscript.id"
            :transcript="activeTranscript"
            @complete="onAnimationComplete"
          />
        </div>
        <TabsList
          aria-label="features"
          :class="[
            'absolute bottom-6 left-1/2 -translate-x-1/2 flex items-center p-1 rounded-md border border-white/10',
            'transition-transform duration-700 delay-300',
            isVisible ? 'translate-y-0' : 'translate-y-12',
          ]"
          style="transition-timing-function: cubic-bezier(0.16, 1, 0.3, 1)"
        >
          <TabsTrigger
            v-for="transcript in terminalTranscripts"
            :key="transcript.id"
            :value="transcript.id"
          >
            {{ transcript.label }}
          </TabsTrigger>
        </TabsList>
      </TabsRoot>
    </div>
  </section>
</template>

<style scoped></style>
