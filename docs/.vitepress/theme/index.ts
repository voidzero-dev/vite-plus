import type { Theme } from 'vitepress'
// note: import the specific variant directly!
import BaseTheme from '@voidzero-dev/vitepress-theme/src/viteplus'
import './styles.css'

export default {
    extends: BaseTheme,
    // Layout
} satisfies Theme
