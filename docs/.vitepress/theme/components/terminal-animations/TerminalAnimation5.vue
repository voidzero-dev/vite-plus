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
    .typeString(`<span>$ vite test</span>`)
    .pauseFor(200)
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pasteString(
      `<div class="block"><span class="text-vite">VITE+ v1.0.0</span> <span class="text-aqua">test</span> RUN ~/vite-plus-demo</div>`,
    )
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pauseFor(300)
    .pasteString(
      `<div class="block"><span class="text-zest">✓</span> test/hello.spec.ts <span class="text-grey">(1 test)</span> <span class="text-zest">1ms</span></div>`,
    )
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pasteString(
      `<div class="block text-grey">Test Files <span class="text-zest">1 passed</span> <span class="text-grey">(1)</span></div>`,
    )
    .pasteString(
      `<div class="block text-grey">Tests <span class="text-zest">1 passed</span> <span class="text-grey">(1)</span></div>`,
    )
    .pasteString(
      `<div class="block text-grey">Start at <span class="text-white">00:13:44</span></div>`,
    )
    .pasteString(
      `<div class="block text-grey">Duration <span class="text-white">199ms</span> <span class="text-grey">(transform 13ms, setup 0ms, collect 8ms, tests 1ms, environment 0ms, prepare 33ms)</span></div>`,
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
