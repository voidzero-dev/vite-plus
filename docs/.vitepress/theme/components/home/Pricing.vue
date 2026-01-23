<script setup lang="ts"></script>

<template>
  <section
    class="wrapper border-t wrapper--ticks grid grid-cols-1 lg:grid-cols-3 divide-x-0 lg:divide-x divide-stroke h-auto lg:h-96"
  >
    <div class="p-5 lg:p-10 flex flex-col justify-start lg:justify-between gap-5 lg:gap-0">
      <h3 class="text-pretty">License &amp; Pricing*</h3>
      <p>Vite+ will be source-available and offers a generous free tier.</p>
    </div>
    <div
      class="col-span-1 lg:col-span-2 grid grid-cols-1 lg:grid-cols-3 divide-y lg:divide-y-0 lg:divide-x divide-stroke h-auto lg:h-full"
    >
      <div class="px-5 py-6 lg:px-7 lg:py-10 flex flex-col gap-3">
        <span class="text-grey text-xs font-mono uppercase tracking-wide pb-0 lg:pb-8"
          >Community</span
        >
        <h4 class="font-sans">Free</h4>
        <p class="text-base text-pretty">
          For open source, non-commercial use, and small businesses.
        </p>
      </div>
      <div class="px-5 py-6 lg:px-6 lg:py-10 flex flex-col gap-3">
        <span class="text-grey text-xs font-mono uppercase tracking-wide pb-0 lg:pb-8"
          >Startup</span
        >
        <h4 class="font-sans">
          Flat fee
          <span class="text-grey text-xs font-mono uppercase tracking-wide align-super"
            >/ year</span
          >
        </h4>
        <p class="text-base text-pretty">
          Simple, flat annual recurring license fee for growing businesses that need to move fast.
        </p>
      </div>
      <div class="px-5 py-6 lg:px-6 lg:py-10 flex flex-col justify-between h-full">
        <div class="flex flex-col gap-3">
          <span class="text-grey text-xs font-mono uppercase tracking-wide pb-0 lg:pb-8"
            >Enterprise</span
          >
          <h4 class="font-sans">Custom</h4>
          <p class="text-base text-pretty">
            For businesses with large teams and complex requirements. Support & SLA available.
          </p>
        </div>
        <p class="text-xs pt-8">* Pricing & tier thresholds are tentative and subject to change.</p>
      </div>
    </div>
  </section>
</template>

<style scoped></style>
