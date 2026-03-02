<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';

const features = [
  { id: 'feature-dev-build', label: 'dev & build' },
  { id: 'feature-test', label: 'test' },
  { id: 'feature-lint', label: 'lint' },
  { id: 'feature-format', label: 'fmt' },
  { id: 'feature-run', label: 'run' },
  { id: 'feature-ui', label: 'ui' },
  { id: 'feature-lib', label: 'lib' },
];

const activeSection = ref('feature-dev-build');
const underlineStyle = ref({ left: '0px', width: '0px' });
const listItems = ref<HTMLElement[]>([]);
let scrollTimeout: number | null = null;

const scrollToSection = (e: Event, id: string) => {
  e.preventDefault();
  e.stopPropagation();

  const element = document.getElementById(id);
  if (!element) {
    return;
  }

  // Get the toolbar height to offset the scroll
  const toolbar = (e.currentTarget as HTMLElement).closest('section');
  const toolbarHeight = toolbar?.offsetHeight || 0;

  // Calculate position to scroll to
  const elementPosition = element.getBoundingClientRect().top + window.scrollY;
  const offsetPosition = elementPosition - toolbarHeight;

  // Use custom smooth scroll with requestAnimationFrame for guaranteed smooth behavior
  const startPosition = window.scrollY;
  const distance = offsetPosition - startPosition;
  const duration = 800; // ms
  let startTime: number | null = null;

  const animation = (currentTime: number) => {
    if (startTime === null) {
      startTime = currentTime;
    }
    const timeElapsed = currentTime - startTime;
    const progress = Math.min(timeElapsed / duration, 1);

    // Easing function (easeInOutCubic)
    const ease =
      progress < 0.5 ? 4 * progress * progress * progress : 1 - Math.pow(-2 * progress + 2, 3) / 2;

    window.scrollTo(0, startPosition + distance * ease);

    if (progress < 1) {
      requestAnimationFrame(animation);
    }
  };

  requestAnimationFrame(animation);
};

const updateUnderlinePosition = () => {
  const activeIndex = features.findIndex((f) => f.id === activeSection.value);
  if (activeIndex >= 0 && listItems.value[activeIndex]) {
    const activeItem = listItems.value[activeIndex];
    underlineStyle.value = {
      left: `${activeItem.offsetLeft}px`,
      width: `${activeItem.offsetWidth}px`,
    };

    // Auto-scroll the toolbar on mobile to keep active item in view
    const toolbar = activeItem.closest('ul');
    if (toolbar && window.innerWidth < 640) {
      // sm breakpoint
      const itemLeft = activeItem.offsetLeft;
      const itemWidth = activeItem.offsetWidth;
      const toolbarWidth = toolbar.clientWidth;

      // Calculate the center position
      const targetScrollLeft = itemLeft - toolbarWidth / 2 + itemWidth / 2;

      toolbar.scrollTo({
        left: targetScrollLeft,
        behavior: 'smooth',
      });
    }
  }
};

const determineActiveSection = () => {
  // Get all sections and their positions
  const sections = features
    .map((feature) => {
      const element = document.getElementById(feature.id);
      if (!element) {
        return null;
      }

      const rect = element.getBoundingClientRect();
      return {
        id: feature.id,
        top: rect.top,
        bottom: rect.bottom,
        height: rect.height,
      };
    })
    .filter(Boolean);

  // Find the section that's most visible near the top of the viewport
  // We consider a section "active" if it's within 200px of the top
  const threshold = 200;
  let activeId = activeSection.value;

  for (const section of sections) {
    if (!section) {
      continue;
    }

    // If section top is near or above threshold and bottom is below threshold
    if (section.top <= threshold && section.bottom > threshold) {
      activeId = section.id;
      break;
    }
  }

  // If no section found above, check if we're at the very bottom
  if (activeId === activeSection.value) {
    const lastSection = sections[sections.length - 1];
    if (
      lastSection &&
      window.innerHeight + window.scrollY >= document.documentElement.scrollHeight - 100
    ) {
      activeId = lastSection.id;
    }
  }

  if (activeId !== activeSection.value) {
    activeSection.value = activeId;
    updateUnderlinePosition();
  }
};

const handleScroll = () => {
  if (scrollTimeout) {
    window.cancelAnimationFrame(scrollTimeout);
  }

  scrollTimeout = window.requestAnimationFrame(() => {
    determineActiveSection();
  });
};

let observer: IntersectionObserver | null = null;

onMounted(() => {
  // Set up scroll listener for active state tracking
  window.addEventListener('scroll', handleScroll, { passive: true });

  // Initial underline position
  setTimeout(() => {
    updateUnderlinePosition();
    determineActiveSection();
  }, 100);

  // Update on resize
  window.addEventListener('resize', updateUnderlinePosition);
});

onUnmounted(() => {
  window.removeEventListener('scroll', handleScroll);
  window.removeEventListener('resize', updateUnderlinePosition);
  if (scrollTimeout) {
    window.cancelAnimationFrame(scrollTimeout);
  }
});
</script>

<template>
  <div class="wrapper wrapper wrapper--ticks border-t w-full relative z-20"></div>
  <section class="wrapper sticky top-0 border-b bg-primary z-10 overflow-hidden">
    <ul
      class="w-full sm:grid sm:grid-cols-7 flex items-center divide-x divide-nickel relative overflow-x-auto scrollbar-hide touch-none sm:touch-auto select-none sm:select-auto"
    >
      <div
        class="absolute bottom-0 h-0.5 bg-white transition-all duration-300 ease-out"
        :style="underlineStyle"
      />
      <li
        v-for="(feature, index) in features"
        :key="feature.id"
        ref="listItems"
        class="flex-shrink-0"
      >
        <a
          :href="`#${feature.id}`"
          @click="scrollToSection($event, feature.id)"
          class="h-full text-sm font-mono tracking-tight py-4 px-6 sm:px-0 flex justify-center gap-1.5 transition-colors duration-200 whitespace-nowrap"
          :class="activeSection === feature.id ? 'text-white' : 'text-grey'"
        >
          <svg
            v-if="index === 0"
            class="w-4"
            viewBox="0 0 17 16"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M3.26172 12.6653C3.26172 13.0335 3.5602 13.332 3.92839 13.332H4.59505C4.96324 13.332 5.26172 13.0335 5.26172 12.6653V5.9987C5.26172 5.6305 4.96324 5.33203 4.59505 5.33203H3.92839C3.5602 5.33203 3.26172 5.6305 3.26172 5.9987V12.6653Z"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linecap="square"
            />
            <path
              d="M7.92847 12.592C7.92847 13.0013 8.22693 13.333 8.59513 13.333H9.2618C9.63 13.333 9.92847 13.0013 9.92847 12.592V2.21921C9.92847 1.81001 9.63 1.47827 9.2618 1.47827H8.59513C8.22693 1.47827 7.92847 1.81 7.92847 2.21921V12.592Z"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linecap="square"
            />
            <path
              d="M12.5952 12.6654C12.5952 13.0336 12.8937 13.332 13.2619 13.332H13.9285C14.2967 13.332 14.5952 13.0336 14.5952 12.6654V7.9987C14.5952 7.63056 14.2967 7.33203 13.9285 7.33203H13.2619C12.8937 7.33203 12.5952 7.63056 12.5952 7.9987V12.6654Z"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linecap="square"
            />
          </svg>
          <svg
            v-else-if="index === 1"
            class="w-4"
            viewBox="0 0 17 16"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M9.97939 1.33325V2.99073C9.97939 4.15603 9.97939 4.73868 10.1221 5.29802C10.2647 5.85736 10.5443 6.37094 11.1035 7.39812L11.8613 8.79005C13.2863 11.4076 13.9988 12.7164 13.4143 13.6866L13.4052 13.7014C12.8121 14.6666 11.3033 14.6666 8.28572 14.6666C5.2681 14.6666 3.7593 14.6666 3.16626 13.7014L3.15722 13.6866C2.57269 12.7164 3.28518 11.4076 4.71016 8.79005L5.46792 7.39812C6.02712 6.37094 6.30672 5.85736 6.44938 5.29802C6.59204 4.73868 6.59204 4.15603 6.59204 2.99073V1.33325"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
            />
            <path
              d="M5.6189 1.33325H10.9522"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
            <path
              d="M5.28564 7.70432C5.95231 6.93539 7.01858 7.48965 8.28564 8.21225C9.95231 9.16272 10.9523 8.43345 11.2856 7.74359"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-linecap="round"
            />
          </svg>
          <svg
            v-else-if="index === 2"
            class="w-4"
            viewBox="0 0 17 16"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <g clip-path="url(#clip0_302_1655)">
              <path
                d="M10.1427 1.33325L10.5019 2.92763C10.8065 4.28002 11.8626 5.33613 13.215 5.64077L14.8094 5.99992L13.215 6.35907C11.8626 6.66371 10.8065 7.71979 10.5019 9.07219L10.1427 10.6666L9.78361 9.07219C9.47894 7.71979 8.42287 6.66371 7.07047 6.35907L5.47607 5.99992L7.07047 5.64077C8.42281 5.33613 9.47894 4.28002 9.78361 2.92764L10.1427 1.33325Z"
                :stroke="activeSection === feature.id ? 'white' : '#827A89'"
                stroke-width="1.2"
                stroke-linejoin="round"
              />
              <path
                d="M4.80941 8L5.06595 9.13887C5.28355 10.1048 6.03791 10.8592 7.00387 11.0768L8.14274 11.3333L7.00387 11.5899C6.03791 11.8075 5.28355 12.5618 5.06595 13.5278L4.80941 14.6667L4.55287 13.5278C4.33527 12.5618 3.58091 11.8075 2.61492 11.5899L1.47607 11.3333L2.61492 11.0768C3.58091 10.8592 4.33527 10.1049 4.55287 9.13887L4.80941 8Z"
                :stroke="activeSection === feature.id ? 'white' : '#827A89'"
                stroke-width="1.2"
                stroke-linejoin="round"
              />
            </g>
            <defs>
              <clipPath id="clip0_302_1655">
                <rect width="16" height="16" fill="white" transform="translate(0.142822)" />
              </clipPath>
            </defs>
          </svg>
          <svg
            v-else-if="index === 3"
            class="w-4"
            viewBox="0 0 17 16"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M7.16634 2.66659C7.16634 1.93021 7.76327 1.33325 8.49967 1.33325H9.16634C9.90274 1.33325 10.4997 1.93021 10.4997 2.66659V4.36883C10.4997 5.24394 11.0686 6.01743 11.904 6.27807L12.4287 6.44177C13.2641 6.70239 13.833 7.47592 13.833 8.35099V9.33325C13.833 9.70145 13.5345 9.99992 13.1663 9.99992H4.49967C4.13149 9.99992 3.83301 9.70145 3.83301 9.33325V8.35099C3.83301 7.47592 4.40193 6.70239 5.23733 6.44177L5.76202 6.27807C6.59741 6.01743 7.16634 5.24394 7.16634 4.36883V2.66659Z"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
            />
            <path
              d="M4.50128 10C4.60515 10.8721 4.16364 13.0088 3.1665 14.5786C3.1665 14.5786 10.0281 15.3729 10.9566 11.9623V13.2475C10.9566 13.875 10.9566 14.1887 11.152 14.3837C11.5279 14.7585 13.2926 14.7687 13.669 14.3681C13.8668 14.1575 13.847 13.8546 13.8072 13.2487C13.7418 12.2497 13.562 10.9705 13.068 10"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <svg
            v-else-if="index === 4"
            class="w-4"
            viewBox="0 0 17 16"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M11.1328 8.26328C11.0318 8.68108 10.5545 8.97635 9.6 9.56688C8.6772 10.1377 8.21586 10.4231 7.844 10.3084C7.69029 10.2609 7.55023 10.1709 7.43728 10.0467C7.16406 9.74661 7.16406 9.16441 7.16406 8.00008C7.16406 6.83575 7.16406 6.25353 7.43728 5.95338C7.55023 5.82929 7.69029 5.7392 7.844 5.69177C8.21586 5.57704 8.6772 5.86246 9.6 6.4333C10.5545 7.02381 11.0318 7.31908 11.1328 7.73688C11.1745 7.90935 11.1745 8.09081 11.1328 8.26328Z"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linejoin="round"
            />
            <path
              d="M2.52393 8.00008C2.52393 5.01452 2.52393 3.52174 3.45142 2.59424C4.37892 1.66675 5.8717 1.66675 8.85726 1.66675C11.8428 1.66675 13.3356 1.66675 14.2631 2.59424C15.1906 3.52174 15.1906 5.01452 15.1906 8.00008C15.1906 10.9856 15.1906 12.4784 14.2631 13.4059C13.3356 14.3334 11.8428 14.3334 8.85726 14.3334C5.8717 14.3334 4.37892 14.3334 3.45142 13.4059C2.52393 12.4784 2.52393 10.9856 2.52393 8.00008Z"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
            />
          </svg>
          <svg
            v-else-if="index === 5"
            class="w-4"
            viewBox="0 0 17 16"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M2.38086 8.00008C2.38086 5.01452 2.38086 3.52174 3.30835 2.59424C4.23585 1.66675 5.72863 1.66675 8.71419 1.66675C11.6997 1.66675 13.1925 1.66675 14.1201 2.59424C15.0475 3.52174 15.0475 5.01452 15.0475 8.00008C15.0475 10.9856 15.0475 12.4784 14.1201 13.4059C13.1925 14.3334 11.6997 14.3334 8.71419 14.3334C5.72863 14.3334 4.23585 14.3334 3.30835 13.4059C2.38086 12.4784 2.38086 10.9856 2.38086 8.00008Z"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linejoin="round"
            />
            <path
              d="M6.38086 6.66675C5.82857 6.66675 5.38086 6.21903 5.38086 5.66675C5.38086 5.11446 5.82857 4.66675 6.38086 4.66675C6.93315 4.66675 7.38086 5.11446 7.38086 5.66675C7.38086 6.21903 6.93315 6.66675 6.38086 6.66675Z"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
            />
            <path
              d="M11.0474 11.3333C11.5996 11.3333 12.0474 10.8855 12.0474 10.3333C12.0474 9.78099 11.5996 9.33325 11.0474 9.33325C10.4951 9.33325 10.0474 9.78099 10.0474 10.3333C10.0474 10.8855 10.4951 11.3333 11.0474 11.3333Z"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
            />
            <path
              d="M7.38086 5.66675H12.0475"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linecap="round"
            />
            <path
              d="M10.0475 10.3333H5.38086"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linecap="round"
            />
          </svg>
          <svg
            v-else-if="index === 6"
            class="w-4"
            viewBox="0 0 17 16"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M11.4131 7H5.72946C4.00843 7 3.14792 7 2.75224 7.5684C2.35658 8.1368 2.65065 8.95053 3.2388 10.578L3.96159 12.578C4.26826 13.4265 4.42159 13.8509 4.76389 14.0921C5.10619 14.3333 5.55488 14.3333 6.45224 14.3333H10.6904C11.5877 14.3333 12.0364 14.3333 12.3787 14.0921C12.721 13.8509 12.8743 13.4265 13.181 12.578L13.9038 10.578C14.492 8.95053 14.786 8.1368 14.3904 7.5684C13.9947 7 13.1342 7 11.4131 7Z"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linecap="square"
            />
            <path
              d="M13.2379 5.33325C13.2379 5.02263 13.2379 4.86731 13.1871 4.7448C13.1195 4.58145 12.9897 4.45166 12.8263 4.384C12.7038 4.33325 12.5485 4.33325 12.2379 4.33325H4.90454C4.59391 4.33325 4.4386 4.33325 4.31609 4.384C4.15273 4.45166 4.02295 4.58145 3.95529 4.7448C3.90454 4.86731 3.90454 5.02263 3.90454 5.33325"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
            <path
              d="M11.5713 2.66675C11.5713 2.35612 11.5713 2.20081 11.5206 2.07829C11.4529 1.91494 11.3231 1.78515 11.1598 1.71749C11.0372 1.66675 10.8819 1.66675 10.5713 1.66675H6.57129C6.26066 1.66675 6.10535 1.66675 5.98284 1.71749C5.81948 1.78515 5.6897 1.91494 5.62204 2.07829C5.57129 2.20081 5.57129 2.35612 5.57129 2.66675"
              :stroke="activeSection === feature.id ? 'white' : '#827A89'"
              stroke-width="1.2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <span>{{ feature.label }}</span>
        </a>
      </li>
    </ul>
  </section>
</template>

<style scoped>
/* Hide scrollbar for Chrome, Safari and Opera */
.scrollbar-hide::-webkit-scrollbar {
  display: none;
}

/* Hide scrollbar for IE, Edge and Firefox */
.scrollbar-hide {
  -ms-overflow-style: none; /* IE and Edge */
  scrollbar-width: none; /* Firefox */
}
</style>
