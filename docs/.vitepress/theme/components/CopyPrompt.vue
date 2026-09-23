<script setup lang="ts">
import { Icon } from '@iconify/vue';
import { computed, onBeforeUnmount, ref, useId } from 'vue';

import { setupPrompt } from '../data/migration-prompts.ts';

const props = withDefaults(
  defineProps<{
    prompt?: string;
    label?: string;
  }>(),
  {
    prompt: '',
    label: 'View Prompt',
  },
);

const promptText = computed(() => props.prompt || setupPrompt);

const titleId = useId();
const dialogEl = ref<HTMLDialogElement | null>(null);
const state = ref<'idle' | 'copied' | 'error'>('idle');
const copyLabel = computed(() =>
  state.value === 'copied'
    ? 'Prompt copied!'
    : state.value === 'error'
      ? 'Could not copy'
      : 'Copy prompt',
);
const copyIcon = computed(() =>
  state.value === 'copied'
    ? 'lucide:check'
    : state.value === 'error'
      ? 'lucide:x'
      : 'lucide:clipboard',
);
let resetTimer: ReturnType<typeof setTimeout> | null = null;

const flash = (next: 'copied' | 'error') => {
  state.value = next;
  if (resetTimer) {
    clearTimeout(resetTimer);
  }
  resetTimer = setTimeout(() => {
    state.value = 'idle';
    resetTimer = null;
  }, 1600);
};

const blurPointerTarget = (event: MouseEvent) => {
  const target = event.currentTarget as HTMLElement;
  requestAnimationFrame(() => target.blur());
};

const copyPrompt = async () => {
  try {
    await navigator.clipboard.writeText(promptText.value);
    flash('copied');
  } catch {
    flash('error');
  }
};

const openView = () => {
  dialogEl.value?.showModal();
};

const closeView = () => {
  dialogEl.value?.close();
};

const onDialogClick = (event: MouseEvent) => {
  if (event.target === dialogEl.value) {
    closeView();
  }
};

onBeforeUnmount(() => {
  if (resetTimer) {
    clearTimeout(resetTimer);
  }
  dialogEl.value?.close();
});
</script>

<template>
  <button
    type="button"
    class="button"
    :aria-label="`${label} for setting up Vite+ with an AI assistant`"
    @mousedown="blurPointerTarget"
    @click="openView"
  >
    <Icon icon="lucide:eye" class="size-4" aria-hidden="true" />
    <span>{{ label }}</span>
  </button>

  <Teleport to="body">
    <dialog
      ref="dialogEl"
      class="m-auto w-[min(48rem,calc(100vw-2rem))] max-h-[min(80vh,36rem)] rounded-xl border border-stroke bg-white p-0 text-primary shadow-xl backdrop:bg-black/40 dark:border-nickel dark:bg-slate dark:text-white"
      :aria-labelledby="titleId"
      @click="onDialogClick"
    >
      <form class="flex max-h-[min(80vh,36rem)] flex-col" method="dialog">
        <header class="flex items-center justify-between gap-4 px-5 pt-4 pb-3">
          <h2 :id="titleId" class="m-0 text-base font-medium">Setup prompt</h2>
          <button
            type="submit"
            class="inline-flex size-8 items-center justify-center rounded-md text-grey hover:bg-beige hover:text-primary dark:text-white dark:hover:bg-slate dark:hover:text-white"
            aria-label="Close"
          >
            <Icon icon="lucide:x" class="size-4" aria-hidden="true" />
          </button>
        </header>
        <pre
          class="m-0 overflow-auto px-5 pb-4 font-mono text-sm leading-relaxed break-words whitespace-pre-wrap"
          >{{ promptText }}</pre>
        <footer class="flex justify-end border-t border-stroke px-5 py-3 dark:border-nickel">
          <button type="button" class="button" @mousedown="blurPointerTarget" @click="copyPrompt">
            <Icon :icon="copyIcon" class="size-4" aria-hidden="true" />
            <span>{{ copyLabel }}</span>
          </button>
        </footer>
      </form>
    </dialog>
  </Teleport>
</template>
