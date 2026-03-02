<template>
  <div class="wrapper flex flex-col justify-start items-center gap-6 pt-14 pb-6">
    <div class="w-full sm:w-2xl flex flex-col justify-start items-center gap-10 px-5 sm:px-0">
      <div class="flex flex-col justify-start items-center gap-4">
        <img src="/icon.svg" alt="Vite+ Logo" class="w-9" />
        <h1 class="text-center text-primary text-balance shine-text">
          <span class="inline-block">The Unified</span>
          <span class="inline-block">Toolchain for the Web</span>
        </h1>
        <p class="self-stretch text-center text-balance text-nickel">
          dev, build, test, lint, format, monorepo caching & more in a single dependency, built for
          scale, speed, and sanity
        </p>
      </div>
      <div class="flex items-center gap-5">
        <a href="/vite/guide" target="_self" class="button button--primary"> Get started </a>
        <!--<a href="#intro" @click="smoothScrollTo($event, 'intro')" class="button">-->
        <!-- <a
          href="https://voidzero.dev/posts/announcing-vite-plus?utm_source=viteplusdev&utm_content=top_learn_more"
          target="_blank"
          rel="noopener noreferrer"
          class="button"
        >
          Learn more
        </a> -->
      </div>
    </div>
  </div>
</template>

<style scoped>
.shine-text {
  background: linear-gradient(
    110deg,
    var(--color-primary) 0%,
    var(--color-primary) 40%,
    #6c3bff 48%,
    #6c3bff 50%,
    #6c3bff 52%,
    var(--color-primary) 60%,
    var(--color-primary) 100%
  );
  background-size: 400% 100%;
  background-position: 100% 0;
  background-clip: text;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  animation: shine 5s ease-in-out 0s 1 forwards;
}

@keyframes shine {
  to {
    background-position: 35% 0;
  }
}
</style>
