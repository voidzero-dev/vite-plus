<script setup lang="ts">
import { Icon } from '@iconify/vue';
import { computed, onBeforeUnmount, ref, useId } from 'vue';

import { migrationPrompt } from '../data/migration-prompts';

// Shared by the homepage and Getting Started guide. Reuse the migration
// prompt so all entry points include the same compatibility review guidance.
const DEFAULT_PROMPT = `I want to use Vite+ in my project. Vite+ is the unified toolchain for the web behind the \`vp\` CLI — one tool combining Vite, Rolldown, Vitest, tsdown, Oxlint, Oxfmt, and Vite Task, plus runtime and package-manager management.

First, read ${__DOCS_ORIGIN__}/llms-full.txt and ${__DOCS_ORIGIN__}/guide to learn Vite+'s commands and configuration. Inspect the worktree and preserve unrelated changes. Determine whether I want a new project or want to migrate or upgrade an existing project. Follow only the matching flow below.

For a new project:

Read ${__DOCS_ORIGIN__}/guide/create and choose the template, target directory, package manager, and intended Vite+ release or preview for my project. Use the target CLI's \`help create\` output to select supported options. Scaffold with \`vp create\`; do not overwrite existing project files.

A global installation is optional. To install the global \`vp\` CLI when it is not already available:
- macOS / Linux: curl -fsSL ${__DOCS_INSTALL_SH_URL__} | bash
- Windows (PowerShell): irm ${__DOCS_INSTALL_PS1_URL__} | iex

Open a new terminal after installation. Follow ${__DOCS_ORIGIN__}/guide/upgrade to select the target release or preview and check \`vp toolchain --global\` before scaffolding.

Without a global installation, use a supported Node.js runtime from the compatibility guide and run \`pnpm dlx --package=vite-plus@<target-version> vp create\` or \`npx --package=vite-plus@<target-version> vp create\`. Replace <target-version> with the intended version. For a preview, use the version from its PR and pass \`--registry=https://registry-bridge.viteplus.dev\` to pnpm or npx before the vp command.

Run \`vp install\`, \`vp check\`, and \`vp test\`, then \`vp build\` for applications or \`vp pack\` for libraries. Without a global CLI, install with the project's package manager and run the local CLI through it, such as \`pnpm exec vp check\` or \`npm exec -- vp check\`. Explain how to use \`vp dev\` for the dev server and \`vp run <task>\` for project scripts or tasks. Report the setup changes, validation results, and any remaining work. Do not commit or push unless I ask.

For an existing project, including one that already uses Vite+, follow these migration or upgrade instructions:

${migrationPrompt}`;

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
