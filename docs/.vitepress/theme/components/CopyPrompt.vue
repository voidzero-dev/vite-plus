<script setup lang="ts">
import { Icon } from '@iconify/vue';
import { computed, onBeforeUnmount, ref, useId } from 'vue';

// Default getting-started prompt handed to an AI coding assistant. Every
// command and URL here is verified against the Getting Started guide and the
// live llms-full.txt docs dump.
const DEFAULT_PROMPT = `I want to use Vite+ in my project. Vite+ is the unified toolchain for the web behind the \`vp\` CLI — one tool combining Vite, Rolldown, Vitest, tsdown, Oxlint, Oxfmt, and Vite Task, plus runtime and package-manager management.

First, read ${__DOCS_ORIGIN__}/llms-full.txt to learn Vite+'s commands and configuration.

Install the \`vp\` CLI if it's not already on the system:
- macOS / Linux: curl -fsSL ${__DOCS_INSTALL_SH_URL__} | bash
- Windows (PowerShell): irm ${__DOCS_INSTALL_PS1_URL__} | iex

Then open a new terminal and run \`vp help\`. To scaffold a new project run \`vp create\`; to move an existing Vite project onto Vite+ run \`vp migrate\`.

Day-to-day commands: \`vp install\` (dependencies), \`vp dev\` (dev server), \`vp check\` (format + lint + type-check), \`vp test\` (tests), and \`vp build\` (production build).

Help me get set up and explain anything I should know.`;

// DEFAULT_PROMPT interpolates the __DOCS_*__ define constants, so it is not a
// static literal and cannot be a withDefaults() default (defineProps is
// hoisted out of setup). Resolve the fallback in promptText instead.
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

const promptText = computed(() => props.prompt || DEFAULT_PROMPT);

const titleId = useId();
const dialogEl = ref<HTMLDialogElement | null>(null);
const state = ref<'idle' | 'copied' | 'error'>('idle');
const copyLabel = computed(() =>
  state.value === 'copied' ? 'Copied!' : state.value === 'error' ? 'Could not copy' : 'Copy Prompt',
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

// The theme draws the `.button` border with an `outline`, but a global reset
// (`button:focus:not(:focus-visible) { outline: none !important }`) strips it
// after a mouse click. The theme only ever uses `.button` on <a> tags, so this
// bites only real <button> elements. For pointer activation (event.detail > 0)
// drop focus so the button returns to its resting state and keeps its border;
// keyboard activation (detail === 0) keeps focus so the a11y focus ring shows.
const blurPointerTarget = (event: MouseEvent) => {
  if (event.detail > 0) {
    (event.currentTarget as HTMLElement | null)?.blur();
  }
};

const copyPrompt = async (event: MouseEvent) => {
  blurPointerTarget(event);
  try {
    await navigator.clipboard.writeText(promptText.value);
    flash('copied');
  } catch {
    flash('error');
  }
};

const openView = (event: MouseEvent) => {
  blurPointerTarget(event);
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
          <button type="button" class="button" @click="copyPrompt">
            <Icon :icon="copyIcon" class="size-4" aria-hidden="true" />
            <span>{{ copyLabel }}</span>
          </button>
        </footer>
      </form>
    </dialog>
  </Teleport>
</template>
