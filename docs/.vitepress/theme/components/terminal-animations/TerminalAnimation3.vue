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
    .typeString(`<span>$ vite lint</span>`)
    .pauseFor(200)
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pasteString(
      `<div class="block text-grey"><span class="text-vite">VITE+ v1.0.0</span> <span class="text-aqua">lint</span></div>`,
    )
    .pauseFor(500)
    .pasteString(
      `<div class="block text-grey">Found <span class="text-white">0 warnings</span> and <span class="text-white">0 errors</span>.</div>`,
    )
    .pasteString(
      `<div class="block text-grey"><span class="text-zest">✓</span> Finished in  <span class="text-white">1ms</span> on <span class="text-white">3 files</span> with <span class="text-white">88 rules</span> using <span class="text-white">10 threads</span>.</div>`,
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
