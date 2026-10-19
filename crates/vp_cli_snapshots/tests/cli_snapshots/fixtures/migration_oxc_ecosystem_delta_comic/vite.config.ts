import { defineConfig } from 'vite-plus';
import type { OxfmtConfig } from 'vite-plus/fmt';
import type { OxlintConfig } from 'vite-plus/lint';

import fmt from './.oxfmtrc.json' with { type: 'json' };
import lint from './.oxlintrc.json' with { type: 'json' };

export default defineConfig({
  fmt: fmt as OxfmtConfig,
  lint: lint as OxlintConfig,
});
