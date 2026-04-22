import fs from 'node:fs';
import fsPromises from 'node:fs/promises';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { readJsonFile, writeJsonFile } from './json.ts';

const VSCODE_SETTINGS = {
  // Set as default over per-lang to avoid conflicts with other formatters
  'editor.defaultFormatter': 'oxc.oxc-vscode',
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
          configPath: './vite.config.ts',
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
    const incoming =
      editorId === 'vscode' && fileName === 'settings.json' && extraVsCodeSettings
        ? { ...extraVsCodeSettings, ...baseIncoming }
        : baseIncoming;
    const filePath = path.join(targetDir, fileName);

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
              `Vite+ adds ${editorConfig.label} settings for the built-in linter and formatter. Merge adds new keys without overwriting existing ones.`,
            ),
          options: [
            {
              label: 'Merge',
              value: 'merge',
              hint: 'Merge new settings into existing file',
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
        // Non-interactive: always merge (safe because existing keys are never overwritten)
        conflictAction = 'merge';
      }

      if (conflictAction === 'merge') {
        mergeAndWriteEditorConfig(filePath, incoming, fileName, displayPath, silent);
      } else {
        if (!silent) {
          prompts.log.info(`Skipped writing ${displayPath}`);
        }
      }
      continue;
    }

    writeJsonFile(filePath, incoming);
    if (!silent) {
      prompts.log.success(`Wrote editor config to ${editorConfig.targetDir}/${fileName}`);
    }
  }
}

function normalizeEditorSelection(editorId: EditorSelection): EditorId[] {
  if (!editorId) {
    return [];
  }
  return [...new Set(Array.isArray(editorId) ? editorId : [editorId])];
}

function mergeAndWriteEditorConfig(
  filePath: string,
  incoming: Record<string, unknown>,
  fileName: string,
  displayPath: string,
  silent = false,
) {
  const existing = readJsonFile(filePath, true);
  const merged = mergeEditorConfigs(existing, incoming, fileName);
  writeJsonFile(filePath, merged);
  if (!silent) {
    prompts.log.success(`Merged editor config into ${displayPath}`);
  }
}

function mergeEditorConfigs(
  existing: Record<string, unknown>,
  incoming: Record<string, unknown>,
  fileName: string,
): Record<string, unknown> {
  if (fileName === 'extensions.json') {
    const existingRecs = Array.isArray(existing['recommendations'])
      ? (existing['recommendations'] as string[])
      : [];
    const incomingRecs = Array.isArray(incoming['recommendations'])
      ? (incoming['recommendations'] as string[])
      : [];
    return {
      ...existing,
      recommendations: [...new Set([...existingRecs, ...incomingRecs])],
    };
  }

  return deepMerge(existing, incoming);
}

function deepMerge(
  target: Record<string, unknown>,
  source: Record<string, unknown>,
): Record<string, unknown> {
  const result = { ...target };
  for (const [key, value] of Object.entries(source)) {
    if (!(key in result)) {
      result[key] = value;
    } else if (
      typeof result[key] === 'object' &&
      result[key] !== null &&
      !Array.isArray(result[key]) &&
      typeof value === 'object' &&
      value !== null &&
      !Array.isArray(value)
    ) {
      result[key] = deepMerge(
        result[key] as Record<string, unknown>,
        value as Record<string, unknown>,
      );
    }
  }
  return result;
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
