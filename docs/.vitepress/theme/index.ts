// note: import the specific variant directly!
import BaseTheme from '@voidzero-dev/vitepress-theme/src/viteplus';
import type { Theme } from 'vitepress';

import CopyPrompt from './components/CopyPrompt.vue';
import Layout from './Layout.vue';
import './styles.css';
import 'virtual:group-icons.css';

export default {
  extends: BaseTheme,
  Layout,
  enhanceApp({ app }) {
    // Globally available so Markdown pages can use <CopyPrompt /> without an import.
    app.component('CopyPrompt', CopyPrompt);
  },
} satisfies Theme;
