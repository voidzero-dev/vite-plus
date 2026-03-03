<script setup lang="ts">
import 'vue3-carousel/carousel.css';
import { Carousel, Slide, Pagination, Navigation } from 'vue3-carousel';

import { testimonials } from '../../data/testimonials';

const carouselConfig = {
  itemsToShow: 'auto',
  wrapAround: true,
  snapAlign: 'start',
};
</script>

<template>
  <section
    class="wrapper border-t wrapper--ticks grid grid-cols-1 md:grid-cols-3 divide-x-0 md:divide-x divide-stroke h-auto md:h-[32.5rem]"
  >
    <div class="flex flex-col gap-0 md:gap-3 divide-y divide-stroke">
      <div
        class="p-5 md:p-10 grow-0 md:grow flex flex-col gap-5 h-72 justify-center md:justify-start"
      >
        <h3 class="text-pretty">Hear from our customers</h3>
        <p class="block md:hidden">
          LLMs already know Vite. Leading prompt-to-app platforms including Bolt, Lovable and Replit
          all default to building user applications using Vite.
        </p>
      </div>
      <div class="px-5 py-6 md:px-10 md:pt-7 md:pb-8 flex flex-col gap-4">
        <h3 class="text-4xl sm:text-5xl font-normal font-sans">500k+</h3>
        <p>Stat label goes here</p>
      </div>
    </div>
    <div class="col-span-1 md:col-span-2 bg-beige/50 border-t border-stroke md:border-t-0">
      <Carousel v-bind="carouselConfig" class="py-10 pb-20 sm:pb-10">
        <Slide v-for="(testimonial, index) in testimonials" :key="index">
          <div
            class="bg-white outline-1 outline-stroke p-5 md:p-10 my-1 ml-5 md:ml-10 rounded w-[80vw] md:w-[25rem] lg:w-[35rem] translate-x-[-1px] flex flex-col justify-between min-h-[60vh] md:min-h-0 h-auto md:h-[23rem]"
          >
            <div class="flex flex-col justify-start items-start gap-10">
              <img
                loading="lazy"
                :src="testimonial.logo"
                :alt="testimonial.logoAlt"
                class="max-w-20 max-h-10 object-contain"
              />
              <span class="text-2xl text-primary font-sans leading-[1.7rem]">
                "{{ testimonial.quote }}"
              </span>
            </div>
            <div class="flex items-center gap-4">
              <img
                loading="lazy"
                :src="testimonial.image"
                :alt="`${testimonial.name} at ${testimonial.company}`"
                class="size-12"
              />
              <p class="text-grey text-sm font-sans leading-tight">
                {{ testimonial.name }}<br />{{ testimonial.title }} @ {{ testimonial.company }}
              </p>
            </div>
          </div>
        </Slide>
        <template #addons>
          <Pagination />
        </template>
      </Carousel>
    </div>
  </section>
</template>

<style>
.carousel__pagination {
}

.carousel__pagination-button {
  height: 6px;
  width: 6px;
  border-radius: 2px;
  background-color: var(--color-stroke);
}
.carousel__pagination-button--active {
  background-color: var(--color-primary);
}
</style>
