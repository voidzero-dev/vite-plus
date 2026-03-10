<script setup lang="ts">
import { TabsList, TabsRoot, TabsTrigger } from 'reka-ui';
import { computed, onMounted, onUnmounted, ref } from 'vue';

import { featureRunTranscripts } from '../../data/feature-run-transcripts';
import TerminalTranscript from './TerminalTranscript.vue';

const AUTO_ADVANCE_DELAY = 2400;

const activeStep = ref(featureRunTranscripts[0].id);
const autoPlayEnabled = ref(true);
const prefersReducedMotion = ref(false);
const hasEnteredViewport = ref(false);
const sectionRef = ref<HTMLElement | null>(null);

let autoAdvanceTimeout: ReturnType<typeof setTimeout> | null = null;
let observer: IntersectionObserver | null = null;
let mediaQuery: MediaQueryList | null = null;

const activeTranscript = computed(
  () =>
    featureRunTranscripts.find((transcript) => transcript.id === activeStep.value) ??
    featureRunTranscripts[0],
);

const clearAutoAdvance = () => {
  if (autoAdvanceTimeout) {
    clearTimeout(autoAdvanceTimeout);
    autoAdvanceTimeout = null;
  }
};

const goToNextStep = () => {
  const currentIndex = featureRunTranscripts.findIndex(
    (transcript) => transcript.id === activeStep.value,
  );
  const nextIndex = (currentIndex + 1) % featureRunTranscripts.length;
  activeStep.value = featureRunTranscripts[nextIndex].id;
};

const onAnimationComplete = () => {
  if (!autoPlayEnabled.value || prefersReducedMotion.value) {
    return;
  }

  clearAutoAdvance();
  autoAdvanceTimeout = setTimeout(() => {
    goToNextStep();
  }, AUTO_ADVANCE_DELAY);
};

const onStepChange = () => {
  clearAutoAdvance();
  if (!prefersReducedMotion.value) {
    autoPlayEnabled.value = true;
    autoAdvanceTimeout = setTimeout(() => {
      goToNextStep();
    }, AUTO_ADVANCE_DELAY);
  }
};

const syncReducedMotionPreference = () => {
  prefersReducedMotion.value = mediaQuery?.matches ?? false;
  if (prefersReducedMotion.value) {
    autoPlayEnabled.value = false;
    clearAutoAdvance();
  }
};

onMounted(() => {
  if (typeof window !== 'undefined' && 'matchMedia' in window) {
    mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    syncReducedMotionPreference();
    if ('addEventListener' in mediaQuery) {
      mediaQuery.addEventListener('change', syncReducedMotionPreference);
    } else {
      mediaQuery.addListener(syncReducedMotionPreference);
    }
  }

  if (!sectionRef.value || typeof IntersectionObserver === 'undefined') {
    hasEnteredViewport.value = true;
    return;
  }

  observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting && !hasEnteredViewport.value) {
          hasEnteredViewport.value = true;
          observer?.disconnect();
        }
      });
    },
    {
      threshold: 0.35,
      rootMargin: '0px',
    },
  );

  observer.observe(sectionRef.value);
});

onUnmounted(() => {
  clearAutoAdvance();
  observer?.disconnect();
  if (!mediaQuery) {
    return;
  }
  if ('removeEventListener' in mediaQuery) {
    mediaQuery.removeEventListener('change', syncReducedMotionPreference);
  } else {
    mediaQuery.removeListener(syncReducedMotionPreference);
  }
});
</script>

<template>
  <div ref="sectionRef" class="feature-run-terminal">
    <TabsRoot v-model="activeStep" @update:modelValue="onStepChange">
      <div
        class="px-4 sm:px-5 py-5 sm:py-6 relative bg-slate rounded-tl rounded-bl outline-1 outline-offset-[2px] outline-white/20"
      >
        <TerminalTranscript
          v-if="hasEnteredViewport"
          :key="activeTranscript.id"
          :transcript="activeTranscript"
          :animate="!prefersReducedMotion"
          @complete="onAnimationComplete"
        />
      </div>
      <TabsList
        aria-label="Vite Task cache examples"
        class="run-step-picker flex items-center p-1 rounded-md border border-white/10 bg-[#111]"
      >
        <TabsTrigger
          v-for="transcript in featureRunTranscripts"
          :key="transcript.id"
          :value="transcript.id"
        >
          {{ transcript.label }}
        </TabsTrigger>
      </TabsList>
    </TabsRoot>
  </div>
</template>

<style scoped>
.feature-run-terminal {
  width: 100%;
  margin-right: 0;
}

.run-step-picker {
  width: fit-content;
  gap: 0.5rem;
  margin: 0.9rem auto 0;
}

:deep(.terminal-copy) {
  min-height: 12.5rem;
  font-size: 0.8125rem;
  line-height: 1.35rem;
}

:deep(.terminal-spacer) {
  height: 0.75rem;
}

@media (min-width: 640px) {
  .feature-run-terminal {
    margin-right: 2.5rem;
  }

  :deep(.terminal-copy) {
    font-size: 0.875rem;
    line-height: 1.5rem;
  }
}
</style>
