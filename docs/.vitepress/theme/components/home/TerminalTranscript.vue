<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';

import type {
  TerminalLine,
  TerminalSegment,
  TerminalTone,
  TerminalTranscript,
} from '../../data/terminal-transcripts';

const props = defineProps<{
  transcript: TerminalTranscript;
  animate?: boolean;
}>();

const emit = defineEmits<{
  complete: [];
}>();

const visibleLineCount = ref(0);
const renderedPrompt = ref('');
const promptFinished = ref(false);
let timers: number[] = [];

const promptText = computed(() => `${props.transcript.prompt ?? '$'} ${props.transcript.command}`);

const visibleLines = computed(() => props.transcript.lines.slice(0, visibleLineCount.value));

const clearTimers = () => {
  timers.forEach((timer) => window.clearTimeout(timer));
  timers = [];
};

const restartAnimation = () => {
  clearTimers();
  if (props.animate === false) {
    visibleLineCount.value = props.transcript.lines.length;
    renderedPrompt.value = promptText.value;
    promptFinished.value = true;
    return;
  }

  visibleLineCount.value = 0;
  renderedPrompt.value = '';
  promptFinished.value = false;

  const characterDelay = 18;
  Array.from(promptText.value).forEach((character, index) => {
    const timer = window.setTimeout(() => {
      renderedPrompt.value += character;
      if (index === promptText.value.length - 1) {
        promptFinished.value = true;
        props.transcript.lines.forEach((_, lineIndex) => {
          const revealTimer = window.setTimeout(
            () => {
              visibleLineCount.value = lineIndex + 1;
              if (lineIndex === props.transcript.lines.length - 1) {
                const completionTimer = window.setTimeout(
                  () => emit('complete'),
                  props.transcript.completionDelay ?? 900,
                );
                timers.push(completionTimer);
              }
            },
            (props.transcript.lineDelay ?? 220) * (lineIndex + 1),
          );
          timers.push(revealTimer);
        });
      }
    }, characterDelay * index);
    timers.push(timer);
  });
};

const lineClass = (line: TerminalLine) => toneClass(line.tone ?? 'base');
const segmentClass = (segment: TerminalSegment) => [
  toneClass(segment.tone ?? 'base'),
  segment.bold ? 'font-bold' : '',
];

const toneClass = (tone: TerminalTone) => {
  switch (tone) {
    case 'muted':
      return 'terminal-tone-muted';
    case 'brand':
      return 'terminal-tone-brand';
    case 'accent':
      return 'terminal-tone-accent';
    case 'success':
      return 'terminal-tone-success';
    case 'warning':
      return 'terminal-tone-warning';
    default:
      return 'terminal-tone-base';
  }
};

watch(
  () => [props.transcript.id, props.animate],
  () => restartAnimation(),
  { immediate: true },
);

onBeforeUnmount(() => clearTimers());
</script>

<template>
  <div class="terminal-copy">
    <div class="terminal-prompt">
      <span class="terminal-tone-base">{{ renderedPrompt }}</span>
      <span v-if="!promptFinished" class="terminal-cursor" aria-hidden="true" />
    </div>
    <div class="terminal-spacer" />
    <TransitionGroup name="terminal-line">
      <div
        v-for="(line, index) in visibleLines"
        :key="`${transcript.id}-${index}`"
        class="terminal-line"
        :class="lineClass(line)"
      >
        <template
          v-for="(segment, segmentIndex) in line.segments"
          :key="`${transcript.id}-${index}-${segmentIndex}`"
        >
          <span :class="segmentClass(segment)">{{ segment.text }}</span>
        </template>
      </div>
    </TransitionGroup>
  </div>
</template>
