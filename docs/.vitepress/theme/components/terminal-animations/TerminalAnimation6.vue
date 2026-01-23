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
    .typeString(`<span>$ vite build</span>`)
    .pauseFor(200)
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pasteString(
      `<div class="block"><span class="text-vite">VITE+ v1.0.0</span> <span class="text-aqua">building for production</span></div>`,
    )
    .pauseFor(100)
    .pasteString(`<div class="block">transforming...</div>`)
    .pauseFor(400)
    .pasteString(
      `<div class="block"><span class="text-zest">✓</span> <span class="text-grey">32 modules transformed...</span></div>`,
    )
    .pauseFor(300)
    .pasteString(`<div class="block">rendering chunks...</div>`)
    .pasteString(`<div class="block">computing gzip size...</div>`)
    .pasteString(
      `<div class="block text-grey"><span class="w-72 inline-block">dist/<span class="text-white">index.html</span></span>&nbsp;&nbsp;0.46 kB | gzip:  0.30 kB</div>`,
    )
    .pasteString(
      `<div class="block text-grey"><span class="w-72 inline-block">dist/assets/<span class="text-vite">react-CHdo91hT.svg</span></span>&nbsp;&nbsp;4.13 kB | gzip:  2.05 kB</div>`,
    )
    .pasteString(
      `<div class="block text-grey"><span class="w-72 inline-block">dist/assets/<span class="text-electric">index-D8b4DHJx.css</span></span>&nbsp;&nbsp;1.39 kB | gzip:  0.71 kB</div>`,
    )
    .pasteString(
      `<div class="block text-grey"><span class="w-72 inline-block">dist/assets/<span class="text-aqua">index-CAl1KfkQ.js</span></span>188.06 kB | gzip: 59.21 kB</div>`,
    )
    .pasteString(`<div class="block"><span class="text-zest">✓ built in 308ms</span></div>`)
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
