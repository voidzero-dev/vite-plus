// To parse this data:
//
//   import { Convert, OxlintConfig } from "./file";
//
//   const oxlintConfig = Convert.toOxlintConfig(json);
//
// These functions will throw an error if the JSON doesn't
// match the expected interface, even if the JSON is valid.

/**
 * Oxlint Configuration File
 *
 * This configuration is aligned with ESLint v8's configuration schema (`eslintrc.json`).
 *
 * Usage: `oxlint -c oxlintrc.json --import-plugin`
 *
 * ::: danger NOTE
 *
 * Only the `.json` format is supported. You can use comments in configuration files.
 *
 * :::
 *
 * Example
 *
 * `.oxlintrc.json`
 *
 * ```json
 * {
 * "$schema": "./node_modules/oxlint/configuration_schema.json",
 * "plugins": ["import", "typescript", "unicorn"],
 * "env": {
 * "browser": true
 * },
 * "globals": {
 * "foo": "readonly"
 * },
 * "settings": {
 * },
 * "rules": {
 * "eqeqeq": "warn",
 * "import/no-cycle": "error",
 * "react/self-closing-comp": ["error", { "html": false }]
 * },
 * "overrides": [
 * {
 * "files": ["*.test.ts", "*.spec.ts"],
 * "rules": {
 * "@typescript-eslint/no-explicit-any": "off"
 * }
 * }
 * ]
 * }
 * ```
 */
export interface OxlintConfig {
  categories?: RuleCategories;
  /**
   * Environments enable and disable collections of global variables.
   */
  env?: { [key: string]: boolean };
  /**
   * Paths of configuration files that this configuration file extends (inherits from). The
   * files
   * are resolved relative to the location of the configuration file that contains the
   * `extends`
   * property. The configuration files are merged from the first to the last, with the last
   * file
   * overriding the previous ones.
   */
  extends?: string[];
  /**
   * Enabled or disabled specific global variables.
   */
  globals?: { [key: string]: GlobalValue };
  /**
   * Globs to ignore during linting. These are resolved from the configuration file path.
   */
  ignorePatterns?: string[];
  /**
   * JS plugins.
   *
   * Note: JS plugins are experimental and not subject to semver.
   * They are not supported in language server at present.
   */
  jsPlugins?: string[] | null;
  /**
   * Add, remove, or otherwise reconfigure rules for specific files or groups of files.
   */
  overrides?: OxlintOverride[];
  /**
   * Enabled built-in plugins for Oxlint.
   * You can view the list of available plugins on
   * [the website](https://oxc.rs/docs/guide/usage/linter/plugins.html#supported-plugins).
   *
   * NOTE: Setting the `plugins` field will overwrite the base set of plugins.
   * The `plugins` array should reflect all of the plugins you want to use.
   */
  plugins?: LintPluginOptionsSchema[] | null;
  /**
   * Example
   *
   * `.oxlintrc.json`
   *
   * ```json
   * {
   * "$schema": "./node_modules/oxlint/configuration_schema.json",
   * "rules": {
   * "eqeqeq": "warn",
   * "import/no-cycle": "error",
   * "prefer-const": ["error", { "ignoreReadBeforeAssign": true }]
   * }
   * }
   * ```
   *
   * See [Oxlint Rules](https://oxc.rs/docs/guide/usage/linter/rules.html) for the list of
   * rules.
   */
  rules?: { [key: string]: any[] | AllowWarnDenyEnum | number };
  settings?: OxlintPluginSettings;
  [property: string]: any;
}

/**
 * Configure an entire category of rules all at once.
 *
 * Rules enabled or disabled this way will be overwritten by individual rules in the `rules`
 * field.
 *
 * Example
 * ```json
 * {
 * "$schema": "./node_modules/oxlint/configuration_schema.json",
 * "categories": {
 * "correctness": "warn"
 * },
 * "rules": {
 * "eslint/no-unused-vars": "error"
 * }
 * }
 * ```
 */
export interface RuleCategories {
  correctness?: AllowWarnDenyEnum | number;
  nursery?: AllowWarnDenyEnum | number;
  pedantic?: AllowWarnDenyEnum | number;
  perf?: AllowWarnDenyEnum | number;
  restriction?: AllowWarnDenyEnum | number;
  style?: AllowWarnDenyEnum | number;
  suspicious?: AllowWarnDenyEnum | number;
}

/**
 * Oxlint rule.
 * - "allow" or "off": Turn off the rule.
 * - "warn": Turn the rule on as a warning (doesn't affect exit code).
 * - "error" or "deny": Turn the rule on as an error (will exit with a failure code).
 */
export type AllowWarnDenyEnum = 'allow' | 'off' | 'warn' | 'error' | 'deny';

export type GlobalValue = 'readonly' | 'writeable' | 'off';

export interface OxlintOverride {
  /**
   * Environments enable and disable collections of global variables.
   */
  env?: { [key: string]: boolean } | null;
  /**
   * A list of glob patterns to override.
   *
   * ## Example
   * `[ "*.test.ts", "*.spec.ts" ]`
   */
  files: string[];
  /**
   * Enabled or disabled specific global variables.
   */
  globals?: { [key: string]: GlobalValue } | null;
  /**
   * JS plugins for this override.
   *
   * Note: JS plugins are experimental and not subject to semver.
   * They are not supported in language server at present.
   */
  jsPlugins?: string[] | null;
  /**
   * Optionally change what plugins are enabled for this override. When
   * omitted, the base config's plugins are used.
   */
  plugins?: LintPluginOptionsSchema[] | null;
  rules?: { [key: string]: any[] | AllowWarnDenyEnum | number };
  [property: string]: any;
}

export type LintPluginOptionsSchema =
  | 'eslint'
  | 'react'
  | 'unicorn'
  | 'typescript'
  | 'oxc'
  | 'import'
  | 'jsdoc'
  | 'jest'
  | 'vitest'
  | 'jsx-a11y'
  | 'nextjs'
  | 'react-perf'
  | 'promise'
  | 'node'
  | 'regex'
  | 'vue';

/**
 * Configure the behavior of linter plugins.
 *
 * Here's an example if you're using Next.js in a monorepo:
 *
 * ```json
 * {
 * "settings": {
 * "next": {
 * "rootDir": "apps/dashboard/"
 * },
 * "react": {
 * "linkComponents": [
 * { "name": "Link", "linkAttribute": "to" }
 * ]
 * },
 * "jsx-a11y": {
 * "components": {
 * "Link": "a",
 * "Button": "button"
 * }
 * }
 * }
 * }
 * ```
 */
export interface OxlintPluginSettings {
  jsdoc?: JSDocPluginSettings;
  'jsx-a11y'?: JSXA11YPluginSettings;
  next?: NextPluginSettings;
  react?: ReactPluginSettings;
  vitest?: VitestPluginSettings;
  [property: string]: any;
}

export interface JSDocPluginSettings {
  /**
   * Only for `require-(yields|returns|description|example|param|throws)` rule
   */
  augmentsExtendsReplacesDocs?: boolean;
  /**
   * Only for `require-param-type` and `require-param-description` rule
   */
  exemptDestructuredRootsFromChecks?: boolean;
  /**
   * For all rules but NOT apply to `empty-tags` rule
   */
  ignoreInternal?: boolean;
  /**
   * For all rules but NOT apply to `check-access` and `empty-tags` rule
   */
  ignorePrivate?: boolean;
  /**
   * Only for `require-(yields|returns|description|example|param|throws)` rule
   */
  ignoreReplacesDocs?: boolean;
  /**
   * Only for `require-(yields|returns|description|example|param|throws)` rule
   */
  implementsReplacesDocs?: boolean;
  /**
   * Only for `require-(yields|returns|description|example|param|throws)` rule
   */
  overrideReplacesDocs?: boolean;
  tagNamePreference?: { [key: string]: boolean | TagNamePreferenceObject | string };
  [property: string]: any;
}

export interface TagNamePreferenceObject {
  message: string;
  replacement?: string;
  [property: string]: any;
}

/**
 * Configure JSX A11y plugin rules.
 *
 * See
 *
 * [eslint-plugin-jsx-a11y](https://github.com/jsx-eslint/eslint-plugin-jsx-a11y#configurations)'s
 * configuration for a full reference.
 */
export interface JSXA11YPluginSettings {
  /**
   * Map of attribute names to their DOM equivalents.
   * This is useful for non-React frameworks that use different attribute names.
   *
   * Example:
   *
   * ```json
   * {
   * "settings": {
   * "jsx-a11y": {
   * "attributes": {
   * "for": ["htmlFor", "for"]
   * }
   * }
   * }
   * }
   * ```
   */
  attributes?: { [key: string]: string[] };
  /**
   * To have your custom components be checked as DOM elements, you can
   * provide a mapping of your component names to the DOM element name.
   *
   * Example:
   *
   * ```json
   * {
   * "settings": {
   * "jsx-a11y": {
   * "components": {
   * "Link": "a",
   * "IconButton": "button"
   * }
   * }
   * }
   * }
   * ```
   */
  components?: { [key: string]: string };
  /**
   * An optional setting that define the prop your code uses to create polymorphic components.
   * This setting will be used to determine the element type in rules that
   * require semantic context.
   *
   * For example, if you set the `polymorphicPropName` to `as`, then this element:
   *
   * ```jsx
   * <Box as="h3">Hello</Box>
   * ```
   *
   * Will be treated as an `h3`. If not set, this component will be treated
   * as a `Box`.
   */
  polymorphicPropName?: null | string;
  [property: string]: any;
}

/**
 * Configure Next.js plugin rules.
 */
export interface NextPluginSettings {
  /**
   * The root directory of the Next.js project.
   *
   * This is particularly useful when you have a monorepo and your Next.js
   * project is in a subfolder.
   *
   * Example:
   *
   * ```json
   * {
   * "settings": {
   * "next": {
   * "rootDir": "apps/dashboard/"
   * }
   * }
   * }
   * ```
   */
  rootDir?: string[] | string;
  [property: string]: any;
}

/**
 * Configure React plugin rules.
 *
 * Derived from
 * [eslint-plugin-react](https://github.com/jsx-eslint/eslint-plugin-react#configuration-legacy-eslintrc-)
 */
export interface ReactPluginSettings {
  /**
   * Components used as alternatives to `<form>` for forms, such as `<Formik>`.
   *
   * Example:
   *
   * ```jsonc
   * {
   * "settings": {
   * "react": {
   * "formComponents": [
   * "CustomForm",
   * // OtherForm is considered a form component and has an endpoint attribute
   * { "name": "OtherForm", "formAttribute": "endpoint" },
   * // allows specifying multiple properties if necessary
   * { "name": "Form", "formAttribute": ["registerEndpoint", "loginEndpoint"] }
   * ]
   * }
   * }
   * }
   * ```
   */
  formComponents?: Array<CustomComponentObject | string>;
  /**
   * Components used as alternatives to `<a>` for linking, such as `<Link>`.
   *
   * Example:
   *
   * ```jsonc
   * {
   * "settings": {
   * "react": {
   * "linkComponents": [
   * "HyperLink",
   * // Use `linkAttribute` for components that use a different prop name
   * // than `href`.
   * { "name": "MyLink", "linkAttribute": "to" },
   * // allows specifying multiple properties if necessary
   * { "name": "Link", "linkAttribute": ["to", "href"] }
   * ]
   * }
   * }
   * }
   * ```
   */
  linkComponents?: Array<CustomComponentObject | string>;
  [property: string]: any;
}

export interface CustomComponentObject {
  attribute?: string;
  name: string;
  attributes?: string[];
  [property: string]: any;
}

/**
 * Configure Vitest plugin rules.
 *
 * See [eslint-plugin-vitest](https://github.com/veritem/eslint-plugin-vitest)'s
 * configuration for a full reference.
 */
export interface VitestPluginSettings {
  /**
   * Whether to enable typecheck mode for Vitest rules.
   * When enabled, some rules will skip certain checks for describe blocks
   * to accommodate TypeScript type checking scenarios.
   */
  typecheck?: boolean;
  [property: string]: any;
}

// Converts JSON strings to/from your types
// and asserts the results of JSON.parse at runtime
export class Convert {
  public static toOxlintConfig(json: string): OxlintConfig {
    return cast(JSON.parse(json), r('OxlintConfig'));
  }

  public static oxlintConfigToJson(value: OxlintConfig): string {
    return JSON.stringify(uncast(value, r('OxlintConfig')), null, 2);
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

function m(additional: any) {
  return { props: [], additional };
}

function r(name: string) {
  return { ref: name };
}

const typeMap: any = {
  OxlintConfig: o(
    [
      { json: 'categories', js: 'categories', typ: u(undefined, r('RuleCategories')) },
      { json: 'env', js: 'env', typ: u(undefined, m(true)) },
      { json: 'extends', js: 'extends', typ: u(undefined, a('')) },
      { json: 'globals', js: 'globals', typ: u(undefined, m(r('GlobalValue'))) },
      { json: 'ignorePatterns', js: 'ignorePatterns', typ: u(undefined, a('')) },
      { json: 'jsPlugins', js: 'jsPlugins', typ: u(undefined, u(a(''), null)) },
      { json: 'overrides', js: 'overrides', typ: u(undefined, a(r('OxlintOverride'))) },
      {
        json: 'plugins',
        js: 'plugins',
        typ: u(undefined, u(a(r('LintPluginOptionsSchema')), null)),
      },
      { json: 'rules', js: 'rules', typ: u(undefined, m(u(a('any'), r('AllowWarnDenyEnum'), 0))) },
      { json: 'settings', js: 'settings', typ: u(undefined, r('OxlintPluginSettings')) },
    ],
    'any',
  ),
  RuleCategories: o(
    [
      { json: 'correctness', js: 'correctness', typ: u(undefined, u(r('AllowWarnDenyEnum'), 0)) },
      { json: 'nursery', js: 'nursery', typ: u(undefined, u(r('AllowWarnDenyEnum'), 0)) },
      { json: 'pedantic', js: 'pedantic', typ: u(undefined, u(r('AllowWarnDenyEnum'), 0)) },
      { json: 'perf', js: 'perf', typ: u(undefined, u(r('AllowWarnDenyEnum'), 0)) },
      { json: 'restriction', js: 'restriction', typ: u(undefined, u(r('AllowWarnDenyEnum'), 0)) },
      { json: 'style', js: 'style', typ: u(undefined, u(r('AllowWarnDenyEnum'), 0)) },
      { json: 'suspicious', js: 'suspicious', typ: u(undefined, u(r('AllowWarnDenyEnum'), 0)) },
    ],
    false,
  ),
  OxlintOverride: o(
    [
      { json: 'env', js: 'env', typ: u(undefined, u(m(true), null)) },
      { json: 'files', js: 'files', typ: a('') },
      { json: 'globals', js: 'globals', typ: u(undefined, u(m(r('GlobalValue')), null)) },
      { json: 'jsPlugins', js: 'jsPlugins', typ: u(undefined, u(a(''), null)) },
      {
        json: 'plugins',
        js: 'plugins',
        typ: u(undefined, u(a(r('LintPluginOptionsSchema')), null)),
      },
      { json: 'rules', js: 'rules', typ: u(undefined, m(u(a('any'), r('AllowWarnDenyEnum'), 0))) },
    ],
    'any',
  ),
  OxlintPluginSettings: o(
    [
      { json: 'jsdoc', js: 'jsdoc', typ: u(undefined, r('JSDocPluginSettings')) },
      { json: 'jsx-a11y', js: 'jsx-a11y', typ: u(undefined, r('JSXA11YPluginSettings')) },
      { json: 'next', js: 'next', typ: u(undefined, r('NextPluginSettings')) },
      { json: 'react', js: 'react', typ: u(undefined, r('ReactPluginSettings')) },
      { json: 'vitest', js: 'vitest', typ: u(undefined, r('VitestPluginSettings')) },
    ],
    'any',
  ),
  JSDocPluginSettings: o(
    [
      {
        json: 'augmentsExtendsReplacesDocs',
        js: 'augmentsExtendsReplacesDocs',
        typ: u(undefined, true),
      },
      {
        json: 'exemptDestructuredRootsFromChecks',
        js: 'exemptDestructuredRootsFromChecks',
        typ: u(undefined, true),
      },
      { json: 'ignoreInternal', js: 'ignoreInternal', typ: u(undefined, true) },
      { json: 'ignorePrivate', js: 'ignorePrivate', typ: u(undefined, true) },
      { json: 'ignoreReplacesDocs', js: 'ignoreReplacesDocs', typ: u(undefined, true) },
      { json: 'implementsReplacesDocs', js: 'implementsReplacesDocs', typ: u(undefined, true) },
      { json: 'overrideReplacesDocs', js: 'overrideReplacesDocs', typ: u(undefined, true) },
      {
        json: 'tagNamePreference',
        js: 'tagNamePreference',
        typ: u(undefined, m(u(true, r('TagNamePreferenceObject'), ''))),
      },
    ],
    'any',
  ),
  TagNamePreferenceObject: o(
    [
      { json: 'message', js: 'message', typ: '' },
      { json: 'replacement', js: 'replacement', typ: u(undefined, '') },
    ],
    'any',
  ),
  JSXA11YPluginSettings: o(
    [
      { json: 'attributes', js: 'attributes', typ: u(undefined, m(a(''))) },
      { json: 'components', js: 'components', typ: u(undefined, m('')) },
      { json: 'polymorphicPropName', js: 'polymorphicPropName', typ: u(undefined, u(null, '')) },
    ],
    'any',
  ),
  NextPluginSettings: o(
    [{ json: 'rootDir', js: 'rootDir', typ: u(undefined, u(a(''), '')) }],
    'any',
  ),
  ReactPluginSettings: o(
    [
      {
        json: 'formComponents',
        js: 'formComponents',
        typ: u(undefined, a(u(r('CustomComponentObject'), ''))),
      },
      {
        json: 'linkComponents',
        js: 'linkComponents',
        typ: u(undefined, a(u(r('CustomComponentObject'), ''))),
      },
    ],
    'any',
  ),
  CustomComponentObject: o(
    [
      { json: 'attribute', js: 'attribute', typ: u(undefined, '') },
      { json: 'name', js: 'name', typ: '' },
      { json: 'attributes', js: 'attributes', typ: u(undefined, a('')) },
    ],
    'any',
  ),
  VitestPluginSettings: o([{ json: 'typecheck', js: 'typecheck', typ: u(undefined, true) }], 'any'),
  AllowWarnDenyEnum: ['allow', 'deny', 'error', 'off', 'warn'],
  GlobalValue: ['off', 'readonly', 'writeable'],
  LintPluginOptionsSchema: [
    'eslint',
    'import',
    'jest',
    'jsdoc',
    'jsx-a11y',
    'nextjs',
    'node',
    'oxc',
    'promise',
    'react',
    'react-perf',
    'regex',
    'typescript',
    'unicorn',
    'vitest',
    'vue',
  ],
};
