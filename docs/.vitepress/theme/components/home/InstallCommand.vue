<script setup lang="ts">
import { Icon } from '@iconify/vue';
import { onBeforeUnmount, ref } from 'vue';

type CommandCard = {
  id: string;
  label: string;
  command: string;
};

const commandCards: CommandCard[] = [
  {
    id: 'unix',
    label: 'macOS / Linux',
    command: 'curl -fsSL https://vite.plus | bash',
  },
  {
    id: 'windows',
    label: 'Windows (PowerShell)',
    command: 'irm https://vite.plus/ps1 | iex',
  },
];

const copiedId = ref<string | null>(null);
let copiedTimer: ReturnType<typeof setTimeout> | null = null;

const flashCopied = (id: string) => {
  copiedId.value = id;
  if (copiedTimer) {
    clearTimeout(copiedTimer);
  }
  copiedTimer = setTimeout(() => {
    copiedId.value = null;
    copiedTimer = null;
  }, 1600);
};

const copyCommand = async (id: string, command: string) => {
  try {
    await navigator.clipboard.writeText(command);
    flashCopied(id);
  } catch {}
};

onBeforeUnmount(() => {
  if (copiedTimer) {
    clearTimeout(copiedTimer);
  }
});
</script>

<template>
  <section
    class="wrapper border-t grid lg:grid-cols-[0.9fr_1.1fr] divide-y lg:divide-y-0 lg:divide-x"
  >
    <div class="px-5 py-6 sm:p-10 flex flex-col gap-4 justify-center">
      <span class="text-grey text-xs font-mono uppercase tracking-wide">Getting started</span>
      <h4>Install vp globally</h4>
      <p class="max-w-[28rem] text-pretty">
        Install Vite+ once, open a new terminal session, then run <code>vp help</code>.
      </p>
      <p class="text-sm text-grey">
        For CI, use
        <a
          class="text-primary underline decoration-stroke underline-offset-4"
          href="https://github.com/voidzero-dev/setup-vp"
          target="_blank"
          rel="noopener noreferrer"
        >
          setup-vp
        </a>
        .
      </p>
    </div>
    <div class="px-5 py-6 sm:p-10 grid gap-4">
      <div
        v-for="card in commandCards"
        :key="card.id"
        class="rounded-xl bg-primary text-white p-5 outline outline-white/10 transition-colors hover:bg-[#1a1a1a]"
      >
        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0 flex-1">
            <div class="text-grey text-xs font-mono uppercase tracking-wide">{{ card.label }}</div>
            <div
              class="mt-3 block overflow-x-auto whitespace-nowrap rounded-md bg-transparent p-0 font-mono text-white outline-none"
            >
              {{ card.command }}
            </div>
          </div>
          <button
            type="button"
            class="shrink-0 inline-flex items-center gap-2 rounded-md border border-white/12 px-3 py-2 text-sm text-grey transition-colors hover:text-white hover:border-white/25"
            :aria-label="`Copy ${card.label} install command`"
            @click="copyCommand(card.id, card.command)"
          >
            <Icon
              :icon="copiedId === card.id ? 'lucide:check' : 'lucide:copy'"
              class="size-4"
              aria-hidden="true"
            />
            <span>{{ copiedId === card.id ? 'Copied' : 'Copy' }}</span>
          </button>
        </div>
      </div>
    </div>
  </section>
</template>
