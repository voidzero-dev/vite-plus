/** CLI options parsed for `vp migrate`. */
export interface MigrationOptions {
  interactive: boolean;
  agent?: string | string[] | false;
  editor?: string | false;
  hooks?: boolean;
  full?: boolean;
}
