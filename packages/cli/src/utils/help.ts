import { stripVTControlCharacters, styleText } from 'node:util';

export type CliDoc = {
  usage?: string;
  summary?: readonly string[] | string;
  sections: readonly CliSection[];
};

export type CliSection = {
  title: string;
  lines?: readonly string[] | string;
  rows?: readonly CliRow[];
};

export type CliRow = {
  label: string;
  description: readonly string[] | string;
};

export type RenderCliDocOptions = {
  color?: boolean;
};

function toLines(value?: readonly string[] | string): string[] {
  if (!value) {
    return [];
  }

  return Array.isArray(value) ? [...value] : [value as string];
}

function visibleLength(value: string): number {
  return stripVTControlCharacters(value).length;
}

function padVisible(value: string, width: number): string {
  const padding = Math.max(0, width - visibleLength(value));
  return `${value}${' '.repeat(padding)}`;
}

function renderRows(rows: readonly CliRow[]): string[] {
  if (rows.length === 0) {
    return [];
  }

  const labelWidth = Math.max(...rows.map((row) => visibleLength(row.label)));
  const output: string[] = [];

  for (const row of rows) {
    const descriptionLines = toLines(row.description);

    if (descriptionLines.length === 0) {
      output.push(`  ${row.label}`);
      continue;
    }

    const [firstLine, ...rest] = descriptionLines;
    output.push(`  ${padVisible(row.label, labelWidth)}  ${firstLine}`);

    for (const line of rest) {
      output.push(`  ${' '.repeat(labelWidth)}  ${line}`);
    }
  }

  return output;
}

function heading(label: string, color: boolean): string {
  return color ? styleText('bold', `${label}:`) : `${label}:`;
}

export function renderCliDoc(doc: CliDoc, options: RenderCliDocOptions = {}): string {
  const color = options.color ?? true;
  const output: string[] = [];

  if (doc.usage) {
    const usage = color ? styleText('bold', doc.usage) : doc.usage;
    output.push(`${heading('Usage', color)} ${usage}`);
  }

  const summaryLines = toLines(doc.summary);
  if (summaryLines.length > 0) {
    if (output.length > 0) {
      output.push('');
    }
    output.push(...summaryLines);
  }

  for (const section of doc.sections) {
    if (output.length > 0) {
      output.push('');
    }
    output.push(heading(section.title, color));

    const lines = toLines(section.lines);
    if (lines.length > 0) {
      output.push(...lines);
    }

    if (section.rows && section.rows.length > 0) {
      output.push(...renderRows(section.rows));
    }
  }

  output.push('');
  return output.join('\n');
}
