// note: import the specific variant directly!
import BaseTheme from '@voidzero-dev/vitepress-theme/src/viteplus';
import type { Theme } from 'vitepress';

import Layout from './Layout.vue';
import './styles.css';
import 'virtual:group-icons.css';

export default {
  extends: BaseTheme,
  Layout,
} satisfies Theme;
