<script setup lang="ts">
import { Icon } from '@iconify/vue';
import { onBeforeUnmount, ref } from 'vue';

// Default getting-started prompt handed to an AI coding assistant. Every
// command and URL here is verified against the Getting Started guide and the
// live llms-full.txt docs dump.
const DEFAULT_PROMPT = `I want to use Vite+ in my project. Vite+ is the unified toolchain for the web behind the \`vp\` CLI — one tool combining Vite, Rolldown, Vitest, tsdown, Oxlint, Oxfmt, and Vite Task, plus runtime and package-manager management.

First, read https://viteplus.dev/llms-full.txt to learn Vite+'s commands and configuration.

Install the \`vp\` CLI:
- macOS / Linux: curl -fsSL https://vite.plus | bash
- Windows (PowerShell): irm https://vite.plus/ps1 | iex

Then open a new terminal and run \`vp help\`. To scaffold a new project run \`vp create\`; to move an existing Vite project onto Vite+ run \`vp migrate\`.

Day-to-day commands: \`vp install\` (dependencies), \`vp dev\` (dev server), \`vp check\` (format + lint + type-check), \`vp test\` (tests), and \`vp build\` (production build).

Help me get set up and explain anything I should know.`;

const props = withDefaults(
  defineProps<{
    prompt?: string;
    label?: string;
  }>(),
  {
    prompt: DEFAULT_PROMPT,
    label: 'Copy Prompt',
  },
);

const state = ref<'idle' | 'copied' | 'error'>('idle');
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

const copyPrompt = async (event: MouseEvent) => {
  // The theme draws the `.button` border with an `outline`, but a global reset
  // (`button:focus:not(:focus-visible) { outline: none !important }`) strips it
  // after a mouse click. The theme only ever uses `.button` on <a> tags, so this
  // bites only real <button> elements. For pointer activation (event.detail > 0)
  // drop focus so the button returns to its resting state and keeps its border;
  // keyboard activation (detail === 0) keeps focus so the a11y focus ring shows.
  if (event.detail > 0) {
    (event.currentTarget as HTMLElement | null)?.blur();
  }
  try {
    await navigator.clipboard.writeText(props.prompt);
    flash('copied');
  } catch {
    flash('error');
  }
};

onBeforeUnmount(() => {
  if (resetTimer) {
    clearTimeout(resetTimer);
  }
});
</script>

<template>
  <button
    type="button"
    class="button"
    :aria-label="`${label} for setting up Vite+ with an AI assistant`"
    @click="copyPrompt"
  >
    <Icon
      :icon="
        state === 'copied'
          ? 'lucide:check'
          : state === 'error'
            ? 'lucide:x'
            : 'lucide:clipboard'
      "
      class="size-4"
      aria-hidden="true"
    />
    <span>{{ state === 'copied' ? 'Copied!' : state === 'error' ? 'Could not copy' : label }}</span>
  </button>
</template>

<style scoped>
.button {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
}
</style>
