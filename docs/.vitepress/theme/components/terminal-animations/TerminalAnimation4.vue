<script setup lang="ts">
import Typewriter from 'typewriter-effect/dist/core';
import { onMounted, onUnmounted } from 'vue';

const props = defineProps<{
  onAnimationComplete?: () => void;
}>();

let typewriter;

onMounted(() => {
  const target = document.getElementById('terminal-code');
  typewriter = new Typewriter(target, {
    loop: false,
    delay: 1,
  });
  typewriter
    .typeString(`<span>$ vite fmt</span>`)
    .pauseFor(200)
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pasteString(
      `<div class="block text-grey"><span class="text-vite">VITE+ v1.0.0</span> <span class="text-aqua">fmt</span></div>`,
    )
    .pauseFor(500)
    .pasteString(
      `<div class="block">src/App.css <span class="text-aqua">0ms</span> <span class="text-grey">(unchanged)</span></div>`,
    )
    .pasteString(`<div class="block">src/App.tsx <span class="text-aqua">1ms</span></div>`)
    .pasteString(
      `<div class="block">src/index.css <span class="text-aqua">0ms</span> <span class="text-grey">(unchanged)</span></div>`,
    )
    .pasteString(`<div class="block">src/main.tsx <span class="text-aqua">1ms</span></div>`)
    .pasteString(
      `<div class="block">src/vite-env.d.ts <span class="text-aqua">0ms</span> <span class="text-grey">(unchanged)</span></div>`,
    )
    .pasteString(
      `<div class="block"><span class="text-zest">✓</span> Formatted <span class="text-aqua">2 files</span> in <span class="text-aqua">2ms</span>.</div>`,
    )
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .callFunction(() => {
      if (props.onAnimationComplete) {
        props.onAnimationComplete();
      }
    })
    .start();
});

onUnmounted(() => {
  if (typewriter) {
    typewriter.stop();
  }
});
</script>

<template>
  <p class="font-mono text-sm text-white leading-[1.5rem]">
    <span id="terminal-code"></span>
  </p>
</template>
