<script setup lang="ts">
import Typewriter from 'typewriter-effect/dist/core';
import { onMounted, onUnmounted } from 'vue';

const props = defineProps<{
  onAnimationComplete?: () => void;
}>();

let typewriter;

onMounted(() => {
  // Get current time for easter egg
  const now = new Date();
  let hours = now.getHours();
  const minutes = now.getMinutes().toString().padStart(2, '0');
  const seconds = now.getSeconds().toString().padStart(2, '0');
  const ampm = hours >= 12 ? 'pm' : 'am';
  hours = hours % 12 || 12;
  const currentTime = `${hours}:${minutes}:${seconds} ${ampm}`;

  const target = document.getElementById('terminal-code');
  typewriter = new Typewriter(target, {
    loop: false,
    delay: 1,
  });
  typewriter
    .typeString(`<span>$ vite dev</span>`)
    .pauseFor(200)
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pasteString(
      `<div class="block text-grey"><span class="text-vite">VITE+ v1.0.0</span> ready in <span class="text-white font-medium">65</span> ms</div>`,
    )
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pauseFor(500)
    .pasteString(
      `<div class="block"><span class="text-vite">  →  Local:   </span><span class="text-aqua">http://localhost:5173/</span></div>`,
    )
    .pasteString(
      `<div class="block text-grey">→  Network: use <span class="text-white font-medium">--host</span> to expose</div>`,
    )
    .pasteString(`<span class="block w-full h-[1rem]"></span>`)
    .pauseFor(1500)
    .pasteString(
      `<div class="block text-grey">${currentTime} <span class="text-aqua">[vite]</span> (client) <span class="text-vite">hmr update</span> /src/App.tsx</div>`,
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
