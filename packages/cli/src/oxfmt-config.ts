// To parse this data:
//
//   import { Convert, OxfmtConfig } from "./file";
//
//   const oxfmtConfig = Convert.toOxfmtConfig(json);
//
// These functions will throw an error if the JSON doesn't
// match the expected interface, even if the JSON is valid.

/**
 * Configuration options for the formatter.
 * Most options are the same as Prettier's options.
 * See also <https://prettier.io/docs/options>
 */
export interface OxfmtConfig {
  /**
   * Include parentheses around a sole arrow function parameter. (Default: "always")
   */
  arrowParens?: ArrowParensConfig | null;
  /**
   * Put the > of a multi-line JSX element at the end of the last line instead of being alone
   * on the next line. (Default: false)
   */
  bracketSameLine?: boolean | null;
  /**
   * Print spaces between brackets in object literals. (Default: true)
   */
  bracketSpacing?: boolean | null;
  /**
   * Control whether formats quoted code embedded in the file. (Default: "auto")
   */
  embeddedLanguageFormatting?: EmbeddedLanguageFormattingConfig | null;
  /**
   * Which end of line characters to apply. (Default: "lf")
   */
  endOfLine?: EndOfLineConfig | null;
  /**
   * Experimental: Sort import statements. Disabled by default.
   */
  experimentalSortImports?: null | SortImportsConfig;
  /**
   * Ignore files matching these glob patterns. Current working directory is used as the root.
   */
  ignorePatterns?: string[] | null;
  /**
   * Use single quotes instead of double quotes in JSX. (Default: false)
   */
  jsxSingleQuote?: boolean | null;
  /**
   * How to wrap object literals when they could fit on one line or span multiple lines.
   * (Default: "preserve")
   * NOTE: In addition to Prettier's "preserve" and "collapse", we also support "always".
   */
  objectWrap?: ObjectWrapConfig | null;
  /**
   * The line length that the printer will wrap on. (Default: 100)
   */
  printWidth?: number | null;
  /**
   * Change when properties in objects are quoted. (Default: "as-needed")
   */
  quoteProps?: QuotePropsConfig | null;
  /**
   * Print semicolons at the ends of statements. (Default: true)
   */
  semi?: boolean | null;
  /**
   * Put each attribute on a new line in JSX. (Default: false)
   */
  singleAttributePerLine?: boolean | null;
  /**
   * Use single quotes instead of double quotes. (Default: false)
   */
  singleQuote?: boolean | null;
  /**
   * Number of spaces per indentation level. (Default: 2)
   */
  tabWidth?: number | null;
  /**
   * Print trailing commas wherever possible. (Default: "all")
   */
  trailingComma?: TrailingCommaConfig | null;
  /**
   * Use tabs for indentation or spaces. (Default: false)
   */
  useTabs?: boolean | null;
  [property: string]: any;
}

export type ArrowParensConfig = 'always' | 'avoid';

export type EmbeddedLanguageFormattingConfig = 'auto' | 'off';

export type EndOfLineConfig = 'lf' | 'crlf' | 'cr';

export interface SortImportsConfig {
  /**
   * Custom groups configuration for organizing imports.
   * Each array element represents a group, and multiple group names in the same array are
   * treated as one.
   * Accepts both `string` and `string[]` as group elements.
   */
  groups?: Array<string[]> | null;
  ignoreCase?: boolean;
  newlinesBetween?: boolean;
  order?: SortOrderConfig | null;
  partitionByComment?: boolean;
  partitionByNewline?: boolean;
  sortSideEffects?: boolean;
  [property: string]: any;
}

export type SortOrderConfig = 'asc' | 'desc';

export type ObjectWrapConfig = 'preserve' | 'collapse' | 'always';

export type QuotePropsConfig = 'as-needed' | 'preserve';

export type TrailingCommaConfig = 'all' | 'es5' | 'none';

// Converts JSON strings to/from your types
// and asserts the results of JSON.parse at runtime
export class Convert {
  public static toOxfmtConfig(json: string): OxfmtConfig {
    return cast(JSON.parse(json), r('OxfmtConfig'));
  }

  public static oxfmtConfigToJson(value: OxfmtConfig): string {
    return JSON.stringify(uncast(value, r('OxfmtConfig')), null, 2);
  }
}

function invalidValue(typ: any, val: any, key: any, parent: any = ''): never {
  const prettyTyp = prettyTypeName(typ);
  const parentText = parent ? ` on ${parent}` : '';
  const keyText = key ? ` for key "${key}"` : '';
  throw Error(
    `Invalid value${keyText}${parentText}. Expected ${prettyTyp} but got ${JSON.stringify(val)}`,
  );
}

function prettyTypeName(typ: any): string {
  if (Array.isArray(typ)) {
    if (typ.length === 2 && typ[0] === undefined) {
      return `an optional ${prettyTypeName(typ[1])}`;
    } else {
      return `one of [${typ
        .map((a) => {
          return prettyTypeName(a);
        })
        .join(', ')}]`;
    }
  } else if (typeof typ === 'object' && typ.literal !== undefined) {
    return typ.literal;
  } else {
    return typeof typ;
  }
}

function jsonToJSProps(typ: any): any {
  if (typ.jsonToJS === undefined) {
    const map: any = {};
    typ.props.forEach((p: any) => (map[p.json] = { key: p.js, typ: p.typ }));
    typ.jsonToJS = map;
  }
  return typ.jsonToJS;
}

function jsToJSONProps(typ: any): any {
  if (typ.jsToJSON === undefined) {
    const map: any = {};
    typ.props.forEach((p: any) => (map[p.js] = { key: p.json, typ: p.typ }));
    typ.jsToJSON = map;
  }
  return typ.jsToJSON;
}

function transform(val: any, typ: any, getProps: any, key: any = '', parent: any = ''): any {
  function transformPrimitive(typ: string, val: any): any {
    if (typeof typ === typeof val) return val;
    return invalidValue(typ, val, key, parent);
  }

  function transformUnion(typs: any[], val: any): any {
    // val must validate against one typ in typs
    const l = typs.length;
    for (let i = 0; i < l; i++) {
      const typ = typs[i];
      try {
        return transform(val, typ, getProps);
      } catch {}
    }
    return invalidValue(typs, val, key, parent);
  }

  function transformEnum(cases: string[], val: any): any {
    if (cases.indexOf(val) !== -1) return val;
    return invalidValue(
      cases.map((a) => {
        return l(a);
      }),
      val,
      key,
      parent,
    );
  }

  function transformArray(typ: any, val: any): any {
    // val must be an array with no invalid elements
    if (!Array.isArray(val)) return invalidValue(l('array'), val, key, parent);
    return val.map((el) => transform(el, typ, getProps));
  }

  function transformDate(val: any): any {
    if (val === null) {
      return null;
    }
    const d = new Date(val);
    if (isNaN(d.valueOf())) {
      return invalidValue(l('Date'), val, key, parent);
    }
    return d;
  }

  function transformObject(props: { [k: string]: any }, additional: any, val: any): any {
    if (val === null || typeof val !== 'object' || Array.isArray(val)) {
      return invalidValue(l(ref || 'object'), val, key, parent);
    }
    const result: any = {};
    Object.getOwnPropertyNames(props).forEach((key) => {
      const prop = props[key];
      const v = Object.prototype.hasOwnProperty.call(val, key) ? val[key] : undefined;
      result[prop.key] = transform(v, prop.typ, getProps, key, ref);
    });
    Object.getOwnPropertyNames(val).forEach((key) => {
      if (!Object.prototype.hasOwnProperty.call(props, key)) {
        result[key] = transform(val[key], additional, getProps, key, ref);
      }
    });
    return result;
  }

  if (typ === 'any') return val;
  if (typ === null) {
    if (val === null) return val;
    return invalidValue(typ, val, key, parent);
  }
  if (typ === false) return invalidValue(typ, val, key, parent);
  let ref: any = undefined;
  while (typeof typ === 'object' && typ.ref !== undefined) {
    ref = typ.ref;
    typ = typeMap[typ.ref];
  }
  if (Array.isArray(typ)) return transformEnum(typ, val);
  if (typeof typ === 'object') {
    return typ.hasOwnProperty('unionMembers')
      ? transformUnion(typ.unionMembers, val)
      : typ.hasOwnProperty('arrayItems')
        ? transformArray(typ.arrayItems, val)
        : typ.hasOwnProperty('props')
          ? transformObject(getProps(typ), typ.additional, val)
          : invalidValue(typ, val, key, parent);
  }
  // Numbers can be parsed by Date but shouldn't be.
  if (typ === Date && typeof val !== 'number') return transformDate(val);
  return transformPrimitive(typ, val);
}

function cast<T>(val: any, typ: any): T {
  return transform(val, typ, jsonToJSProps);
}

function uncast<T>(val: T, typ: any): any {
  return transform(val, typ, jsToJSONProps);
}

function l(typ: any) {
  return { literal: typ };
}

function a(typ: any) {
  return { arrayItems: typ };
}

function u(...typs: any[]) {
  return { unionMembers: typs };
}

function o(props: any[], additional: any) {
  return { props, additional };
}

function r(name: string) {
  return { ref: name };
}

const typeMap: any = {
  OxfmtConfig: o(
    [
      {
        json: 'arrowParens',
        js: 'arrowParens',
        typ: u(undefined, u(r('ArrowParensConfig'), null)),
      },
      { json: 'bracketSameLine', js: 'bracketSameLine', typ: u(undefined, u(true, null)) },
      { json: 'bracketSpacing', js: 'bracketSpacing', typ: u(undefined, u(true, null)) },
      {
        json: 'embeddedLanguageFormatting',
        js: 'embeddedLanguageFormatting',
        typ: u(undefined, u(r('EmbeddedLanguageFormattingConfig'), null)),
      },
      { json: 'endOfLine', js: 'endOfLine', typ: u(undefined, u(r('EndOfLineConfig'), null)) },
      {
        json: 'experimentalSortImports',
        js: 'experimentalSortImports',
        typ: u(undefined, u(null, r('SortImportsConfig'))),
      },
      { json: 'ignorePatterns', js: 'ignorePatterns', typ: u(undefined, u(a(''), null)) },
      { json: 'jsxSingleQuote', js: 'jsxSingleQuote', typ: u(undefined, u(true, null)) },
      { json: 'objectWrap', js: 'objectWrap', typ: u(undefined, u(r('ObjectWrapConfig'), null)) },
      { json: 'printWidth', js: 'printWidth', typ: u(undefined, u(0, null)) },
      { json: 'quoteProps', js: 'quoteProps', typ: u(undefined, u(r('QuotePropsConfig'), null)) },
      { json: 'semi', js: 'semi', typ: u(undefined, u(true, null)) },
      {
        json: 'singleAttributePerLine',
        js: 'singleAttributePerLine',
        typ: u(undefined, u(true, null)),
      },
      { json: 'singleQuote', js: 'singleQuote', typ: u(undefined, u(true, null)) },
      { json: 'tabWidth', js: 'tabWidth', typ: u(undefined, u(0, null)) },
      {
        json: 'trailingComma',
        js: 'trailingComma',
        typ: u(undefined, u(r('TrailingCommaConfig'), null)),
      },
      { json: 'useTabs', js: 'useTabs', typ: u(undefined, u(true, null)) },
    ],
    'any',
  ),
  SortImportsConfig: o(
    [
      { json: 'groups', js: 'groups', typ: u(undefined, u(a(a('')), null)) },
      { json: 'ignoreCase', js: 'ignoreCase', typ: u(undefined, true) },
      { json: 'newlinesBetween', js: 'newlinesBetween', typ: u(undefined, true) },
      { json: 'order', js: 'order', typ: u(undefined, u(r('SortOrderConfig'), null)) },
      { json: 'partitionByComment', js: 'partitionByComment', typ: u(undefined, true) },
      { json: 'partitionByNewline', js: 'partitionByNewline', typ: u(undefined, true) },
      { json: 'sortSideEffects', js: 'sortSideEffects', typ: u(undefined, true) },
    ],
    'any',
  ),
  ArrowParensConfig: ['always', 'avoid'],
  EmbeddedLanguageFormattingConfig: ['auto', 'off'],
  EndOfLineConfig: ['cr', 'crlf', 'lf'],
  SortOrderConfig: ['asc', 'desc'],
  ObjectWrapConfig: ['always', 'collapse', 'preserve'],
  QuotePropsConfig: ['as-needed', 'preserve'],
  TrailingCommaConfig: ['all', 'es5', 'none'],
};
