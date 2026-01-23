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
    .typeString(`<span>$ vite new vite-plus-demo --template react-ts</span>`)
    .pauseFor(500)
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pasteString(`<span class="text-aqua">◇  Scaffolding project in ~/vite-plus-demo</span>`)
    .pauseFor(500)
    .pasteString(`<span class="text-grey block">|</span>`)
    .pasteString(`<span class="text-zest block">└  Done.</span>`)
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pauseFor(500)
    .pasteString(
      `<span class="block"><span class="text-zest">✓</span> Installing dependencies using default package manager: <span class="text-vite">pnpm@v10.16.1</span></span>`,
    )
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pasteString(
      `<div class="block"><span class="text-grey">Progress:</span> resolved 1, reused 0, downloaded 0, added 0</div>`,
    )
    .pasteString(
      `<div class="block"><span class="text-grey">Packages:</span> <span class="text-vite">+31</span></div>`,
    )
    .typeString(
      `<span class="text-grey block">++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++</span>`,
    )
    .pasteString(
      `<div class="block"><span class="text-grey">Progress:</span> resolved 31, reused 31, downloaded 0, added 31, <span class="text-zest">done</span></div>`,
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
