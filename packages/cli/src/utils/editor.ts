import fs from 'node:fs';
import fsPromises from 'node:fs/promises';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import {
  applyEdits,
  type FormattingOptions,
  type JSONPath,
  modify,
  type ModificationOptions,
  parse as parseJsonc,
} from 'jsonc-parser';

import { detectFormattingOptions, writeJsonFile } from './json.ts';

// Language-specific overrides because user-level [lang] settings beat the workspace default
const VSCODE_LANGUAGE_OVERRIDES = {
  '[javascript]': { 'editor.defaultFormatter': 'oxc.oxc-vscode' },
  '[javascriptreact]': { 'editor.defaultFormatter': 'oxc.oxc-vscode' },
  '[typescript]': { 'editor.defaultFormatter': 'oxc.oxc-vscode' },
  '[typescriptreact]': { 'editor.defaultFormatter': 'oxc.oxc-vscode' },
} as const;

const VSCODE_SETTINGS = {
  'editor.defaultFormatter': 'oxc.oxc-vscode',
  ...VSCODE_LANGUAGE_OVERRIDES,
  'oxc.fmt.configPath': './vite.config.ts',
  'editor.formatOnSave': true,
  // Oxfmt does not support partial formatting
  'editor.formatOnSaveMode': 'file',
  'editor.codeActionsOnSave': {
    'source.fixAll.oxc': 'explicit',
  },
} as const;

const VSCODE_EXTENSIONS = {
  recommendations: ['VoidZero.vite-plus-extension-pack'],
} as const;

const ZED_SETTINGS = {
  lsp: {
    oxlint: {
      initialization_options: {
        settings: {
          run: 'onType',
          fixKind: 'safe_fix',
          typeAware: true,
          unusedDisableDirectives: 'deny',
        },
      },
    },
    oxfmt: {
      initialization_options: {
        settings: {
          'fmt.configPath': './vite.config.ts',
          run: 'onSave',
        },
      },
    },
  },
  languages: {
    CSS: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    GraphQL: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    Handlebars: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    HTML: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    JavaScript: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
      code_action: 'source.fixAll.oxc',
    },
    JSX: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    JSON: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    JSON5: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    JSONC: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    Less: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    Markdown: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    MDX: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    SCSS: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    TypeScript: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    TSX: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    'Vue.js': {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
    YAML: {
      format_on_save: 'on',
      prettier: { allowed: false },
      formatter: [{ language_server: { name: 'oxfmt' } }],
    },
  },
} as const;

const JETBRAINS_EXTERNAL_DEPENDENCIES = `<?xml version="1.0" encoding="UTF-8"?>
<project version="4">
  <component name="ExternalDependencies">
    <plugin id="com.github.oxc.project.oxcintellijplugin" />
    <plugin id="intellij.vitejs" />
  </component>
</project>
`;

type EditorConfigValue = Record<string, unknown> | string;

export const EDITORS = [
  {
    id: 'vscode',
    label: 'VSCode',
    targetDir: '.vscode',
    files: {
      'settings.json': VSCODE_SETTINGS as Record<string, unknown>,
      'extensions.json': VSCODE_EXTENSIONS as Record<string, unknown>,
    },
  },
  {
    id: 'zed',
    label: 'Zed',
    targetDir: '.zed',
    files: {
      'settings.json': ZED_SETTINGS as Record<string, unknown>,
    },
  },
  {
    id: 'jetbrains',
    label: 'JetBrains (IntelliJ, WebStorm, etc)',
    targetDir: '.idea',
    files: {
      'externalDependencies.xml': JETBRAINS_EXTERNAL_DEPENDENCIES,
    },
  },
] as const;

export type EditorId = (typeof EDITORS)[number]['id'];
type EditorSelection = EditorId | readonly EditorId[] | undefined;

export async function selectEditor({
  interactive,
  editor,
  onCancel,
}: {
  interactive: boolean;
  editor?: string | false;
  onCancel: () => void;
}): Promise<EditorId | undefined> {
  // Skip entirely if --no-editor is passed
  if (editor === false) {
    return undefined;
  }

  if (interactive && !editor) {
    const editorOptions = EDITORS.map((option) => ({
      label: option.label,
      value: option.id,
      hint: option.targetDir,
    }));
    const otherOption = {
      label: 'Other',
      value: null,
      hint: 'Skip writing editor configs',
    };
    const selectedEditor = await prompts.select({
      message:
        'Which editor are you using?\n  ' +
        styleText(
          'gray',
          'Writes editor config files to enable recommended extensions and Oxlint/Oxfmt integrations.',
        ),
      options: [...editorOptions, otherOption],
      initialValue: 'vscode',
    });

    if (prompts.isCancel(selectedEditor)) {
      onCancel();
      return undefined;
    }

    if (selectedEditor === null) {
      return undefined;
    }
    return resolveEditorId(selectedEditor);
  }

  if (editor) {
    return resolveEditorId(editor);
  }

  return undefined;
}

export async function selectEditors({
  interactive,
  editor,
  onCancel,
}: {
  interactive: boolean;
  editor?: string | false;
  onCancel: () => void;
}): Promise<EditorId[] | undefined> {
  if (editor === false) {
    return undefined;
  }

  if (interactive && !editor) {
    const selectedEditors = await prompts.multiselect({
      message:
        'Which editors are you using?\n  ' +
        styleText(
          'gray',
          'Writes editor config files to enable recommended extensions and Oxlint/Oxfmt integrations.',
        ),
      options: EDITORS.map((option) => ({
        label: option.label,
        value: option.id,
        hint: option.targetDir,
      })),
      initialValues: ['vscode'],
      required: false,
    });

    if (prompts.isCancel(selectedEditors)) {
      onCancel();
      return undefined;
    }

    return selectedEditors.length === 0 ? undefined : resolveEditorIds(selectedEditors);
  }

  if (editor) {
    const editorId = resolveEditorId(editor);
    return editorId ? [editorId] : undefined;
  }

  return undefined;
}

export function detectExistingEditor(projectRoot: string): EditorId | undefined {
  return detectExistingEditors(projectRoot)?.[0];
}

export function detectExistingEditors(projectRoot: string): EditorId[] | undefined {
  const editors: EditorId[] = [];
  for (const option of EDITORS) {
    for (const fileName of Object.keys(option.files)) {
      const filePath = path.join(projectRoot, option.targetDir, fileName);
      if (fs.existsSync(filePath)) {
        editors.push(option.id);
        break;
      }
    }
  }
  return editors.length === 0 ? undefined : editors;
}

export interface EditorConflictInfo {
  fileName: string;
  displayPath: string;
}

/**
 * Detect editor config files that would conflict (already exist).
 * Read-only — does not write or modify any files.
 */
export function detectEditorConflicts({
  projectRoot,
  editorId,
}: {
  projectRoot: string;
  editorId: EditorId | undefined;
}): EditorConflictInfo[] {
  if (!editorId) {
    return [];
  }

  const editorConfig = EDITORS.find((e) => e.id === editorId);
  if (!editorConfig) {
    return [];
  }

  const conflicts: EditorConflictInfo[] = [];
  for (const fileName of Object.keys(editorConfig.files)) {
    const filePath = path.join(projectRoot, editorConfig.targetDir, fileName);
    if (fs.existsSync(filePath)) {
      conflicts.push({
        fileName,
        displayPath: `${editorConfig.targetDir}/${fileName}`,
      });
    }
  }

  return conflicts;
}

export async function writeEditorConfigs({
  projectRoot,
  editorId,
  interactive,
  conflictDecisions,
  silent = false,
  extraVsCodeSettings,
}: {
  projectRoot: string;
  editorId: EditorSelection;
  interactive: boolean;
  conflictDecisions?: Map<string, 'merge' | 'skip'>;
  silent?: boolean;
  extraVsCodeSettings?: Record<string, string>;
}) {
  const editorIds = normalizeEditorSelection(editorId);
  if (editorIds.length === 0) {
    return;
  }

  for (const currentEditorId of editorIds) {
    await writeEditorConfig({
      projectRoot,
      editorId: currentEditorId,
      interactive,
      conflictDecisions,
      silent,
      extraVsCodeSettings,
    });
  }
}

async function writeEditorConfig({
  projectRoot,
  editorId,
  interactive,
  conflictDecisions,
  silent,
  extraVsCodeSettings,
}: {
  projectRoot: string;
  editorId: EditorId;
  interactive: boolean;
  conflictDecisions?: Map<string, 'merge' | 'skip'>;
  silent: boolean;
  extraVsCodeSettings?: Record<string, string>;
}) {
  const editorConfig = EDITORS.find((e) => e.id === editorId);
  if (!editorConfig) {
    return;
  }

  const targetDir = path.join(projectRoot, editorConfig.targetDir);
  await fsPromises.mkdir(targetDir, { recursive: true });

  for (const [fileName, baseIncoming] of Object.entries(editorConfig.files)) {
    const incoming: EditorConfigValue =
      editorId === 'vscode' && fileName === 'settings.json' && extraVsCodeSettings
        ? { ...extraVsCodeSettings, ...baseIncoming }
        : baseIncoming;
    const filePath = path.join(targetDir, fileName);
    const jsonFormat = isJsonLikeFile(fileName);

    if (fs.existsSync(filePath)) {
      const displayPath = `${editorConfig.targetDir}/${fileName}`;

      // Determine conflict action from pre-resolved decisions, interactive prompt, or default
      let conflictAction: 'merge' | 'skip';
      const preResolved = conflictDecisions?.get(displayPath) ?? conflictDecisions?.get(fileName);
      if (preResolved) {
        conflictAction = preResolved;
      } else if (interactive) {
        const action = await prompts.select({
          message:
            `${displayPath} already exists.\n  ` +
            styleText(
              'gray',
              jsonFormat
                ? `Vite+ adds ${editorConfig.label} settings for the built-in linter and formatter. Merge adds new keys without overwriting existing ones.`
                : `Vite+ adds ${editorConfig.label} settings for the built-in linter and formatter. Overwrite replaces the existing file with the generated config.`,
            ),
          options: [
            {
              label: jsonFormat ? 'Merge' : 'Overwrite',
              value: 'merge',
              hint: jsonFormat
                ? 'Merge new settings into existing file'
                : 'Replace existing file with generated config',
            },
            {
              label: 'Skip',
              value: 'skip',
              hint: 'Leave existing file unchanged',
            },
          ],
          initialValue: 'skip',
        });
        conflictAction = prompts.isCancel(action) || action === 'skip' ? 'skip' : 'merge';
      } else {
        // Non-interactive: merge JSON safely, skip non-JSON to avoid destructive overwrite.
        conflictAction = jsonFormat ? 'merge' : 'skip';
      }

      if (conflictAction === 'merge') {
        if (jsonFormat) {
          if (!isPlainObject(incoming)) {
            throw new Error(`Cannot merge editor config: ${displayPath} incoming value is not JSON`);
          }
          mergeAndWriteEditorConfig(filePath, incoming, fileName, displayPath, silent);
        } else {
          writeTextEditorConfig(filePath, incoming, displayPath, silent);
        }
      } else {
        if (!silent) {
          prompts.log.info(`Skipped writing ${displayPath}`);
        }
      }
      continue;
    }

    if (jsonFormat) {
      if (!isPlainObject(incoming)) {
        throw new Error(`Cannot write editor config: ${editorConfig.targetDir}/${fileName} must be JSON`);
      }
      writeJsonFile(filePath, incoming);
    } else {
      writeTextEditorConfig(filePath, incoming, `${editorConfig.targetDir}/${fileName}`, silent);
    }
    if (!silent) {
      prompts.log.success(`Wrote editor config to ${editorConfig.targetDir}/${fileName}`);
    }
  }
}

function isJsonLikeFile(fileName: string): boolean {
  const ext = path.extname(fileName).toLowerCase();
  return ext === '.json' || ext === '.jsonc';
}

function writeTextEditorConfig(
  filePath: string,
  incoming: EditorConfigValue,
  displayPath: string,
  silent = false,
) {
  if (typeof incoming !== 'string') {
    throw new Error(`Cannot write editor config: ${displayPath} must be text content`);
  }

  const existingText = fs.existsSync(filePath) ? fs.readFileSync(filePath, 'utf-8') : undefined;
  if (existingText === incoming) {
    if (!silent) {
      prompts.log.info(`No changes needed for ${displayPath}`);
    }
    return;
  }

  fs.writeFileSync(filePath, incoming, 'utf-8');
}

function normalizeEditorSelection(editorId: EditorSelection): EditorId[] {
  if (!editorId) {
    return [];
  }
  return [...new Set(Array.isArray(editorId) ? editorId : [editorId])];
}

/**
 * Merge incoming settings into an existing editor JSON/JSONC file by patching the
 * original text with `jsonc-parser` instead of re-serializing a merged object.
 * This preserves comments, key order, trailing commas, and untouched formatting.
 * Existing values always win; only missing keys/branches are inserted.
 */
function mergeAndWriteEditorConfig(
  filePath: string,
  incoming: Record<string, unknown>,
  fileName: string,
  displayPath: string,
  silent = false,
) {
  const originalText = fs.readFileSync(filePath, 'utf-8');
  const existing = parseJsonc(originalText) as unknown;
  if (!isPlainObject(existing)) {
    throw new Error(`Cannot merge editor config: ${displayPath} is not a JSON object`);
  }

  const formattingOptions = detectFormattingOptions(originalText);
  const newText =
    fileName === 'extensions.json'
      ? mergeExtensionsText(originalText, existing, incoming, formattingOptions)
      : mergeSettingsText(originalText, existing, incoming, formattingOptions);

  // Do not rewrite when the merge produced no changes (keeps the operation idempotent).
  if (newText === originalText) {
    if (!silent) {
      prompts.log.info(`No changes needed for ${displayPath}`);
    }
    return;
  }

  fs.writeFileSync(filePath, newText, 'utf-8');
  if (!silent) {
    prompts.log.success(`Merged editor config into ${displayPath}`);
  }
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Apply a single `jsonc-parser` modification to `text` and return the patched text. */
function applyJsoncEdit(
  text: string,
  path: JSONPath,
  value: unknown,
  options: ModificationOptions,
): string {
  return applyEdits(text, modify(text, path, value, options));
}

/**
 * Deep-merge missing keys from `incoming` into the existing text. Inserts a whole
 * branch when it is absent, and recurses only when both sides are non-array objects
 * so comments inside existing branches are preserved.
 */
function mergeSettingsText(
  text: string,
  existing: Record<string, unknown>,
  incoming: Record<string, unknown>,
  formattingOptions: FormattingOptions,
): string {
  let currentText = text;
  const insertMissing = (
    existingNode: Record<string, unknown>,
    incomingNode: Record<string, unknown>,
    basePath: JSONPath,
  ) => {
    for (const [key, value] of Object.entries(incomingNode)) {
      const fullPath = [...basePath, key];
      if (!(key in existingNode)) {
        currentText = applyJsoncEdit(currentText, fullPath, value, { formattingOptions });
      } else if (isPlainObject(existingNode[key]) && isPlainObject(value)) {
        insertMissing(existingNode[key], value, fullPath);
      }
      // Otherwise the existing value wins and is left untouched.
    }
  };
  insertMissing(existing, incoming, []);
  return currentText;
}

/**
 * For `extensions.json`, append missing recommendations without rebuilding the array,
 * so comments inside the array survive. Existing entries always win.
 */
function mergeExtensionsText(
  text: string,
  existing: Record<string, unknown>,
  incoming: Record<string, unknown>,
  formattingOptions: FormattingOptions,
): string {
  const incomingRecs = Array.isArray(incoming['recommendations'])
    ? (incoming['recommendations'] as unknown[])
    : [];
  const existingValue = existing['recommendations'];

  // No existing recommendations key: insert the incoming array as-is.
  if (!('recommendations' in existing)) {
    return applyJsoncEdit(text, ['recommendations'], incomingRecs, { formattingOptions });
  }

  // Unexpected non-array value: existing user value wins, leave it untouched.
  if (!Array.isArray(existingValue)) {
    return text;
  }

  const existingRecs = new Set<unknown>(existingValue);
  let currentText = text;
  let nextIndex = existingValue.length;
  for (const rec of incomingRecs) {
    if (existingRecs.has(rec)) {
      continue;
    }
    currentText = applyJsoncEdit(currentText, ['recommendations', nextIndex], rec, {
      formattingOptions,
      isArrayInsertion: true,
    });
    nextIndex++;
  }
  return currentText;
}

function resolveEditorId(editor: string): EditorId | undefined {
  const normalized = editor.trim().toLowerCase();
  const match = EDITORS.find(
    (option) => option.id === normalized || option.label.toLowerCase() === normalized,
  );
  return match?.id;
}

function resolveEditorIds(editors: readonly string[]): EditorId[] | undefined {
  const editorIds = editors.flatMap((editor) => {
    const editorId = resolveEditorId(editor);
    return editorId ? [editorId] : [];
  });
  const uniqueEditorIds = [...new Set(editorIds)];
  return uniqueEditorIds.length === 0 ? undefined : uniqueEditorIds;
}
