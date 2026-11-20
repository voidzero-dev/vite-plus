// Build-time constants injected via vite.define in config.mts. They point at
// the current deploy's origin (production, main preview, or PR staging). The
// dunder names follow the Vite convention for compile-time replaced globals.
// oxlint-disable-next-line no-underscore-dangle
declare const __DOCS_ORIGIN__: string;
// oxlint-disable-next-line no-underscore-dangle
declare const __DOCS_INSTALL_SH_URL__: string;
// oxlint-disable-next-line no-underscore-dangle
declare const __DOCS_INSTALL_PS1_URL__: string;

// The theme re-exports this named type from an SFC, which plain TypeScript cannot read.
// Keep this specific shim before the generic Vue module declaration.
declare module '*VPSponsors.vue' {
  import type { SponsorTier } from '@voidzero-dev/vitepress-theme/src/types/theme-config';
  import type { DefineComponent } from 'vue';

  export type Sponsors = Omit<SponsorTier, 'tier'> & { tier?: string };
  const component: DefineComponent<{}, {}, unknown>;
  export default component;
}

// Vue SFC module declaration
declare module '*.vue' {
  import type { DefineComponent } from 'vue';
  const component: DefineComponent<{}, {}, unknown>;
  export default component;
}

// CSS module declarations
declare module '*.css' {}

// Asset module declarations
declare module '*.riv' {
  const src: string;
  export default src;
}

declare module '*.svg' {
  const src: string;
  export default src;
}

declare module '*.png' {
  const src: string;
  export default src;
}

declare module '*.jpg' {
  const src: string;
  export default src;
}
