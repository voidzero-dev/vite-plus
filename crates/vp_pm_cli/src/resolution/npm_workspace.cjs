// This preload is scoped to the npm process that performs the edit. Loading
// through the original launcher preserves its Node version and environment.
const fs = require('fs');
const path = require('path');
const Module = require('module');

async function plan(Config, options, npm, member) {
  async function loadConfig(root) {
    const config = new Config({ ...options, env: { ...process.env } });
    if (root)
      config.loadLocalPrefix = () => {
        config.localPrefix = root;
      };
    await config.load();
    return config;
  }
  function nativeScope(config) {
    return (
      config.get('global') ||
      config.get('location') === 'global' ||
      config.find('prefix') === 'cli' ||
      config.get('workspaces') === false ||
      !config.get('include-workspace-root')
    );
  }
  let config = await loadConfig();
  if (nativeScope(config)) return null;

  const mapWorkspaces = npm('@npmcli/map-workspaces');
  member = fs.realpathSync(member);
  function readManifest(dir) {
    try {
      return JSON.parse(
        fs.readFileSync(path.join(dir, 'package.json'), 'utf8').replace(/^\uFEFF/, ''),
      );
    } catch (error) {
      if (error.code !== 'ENOENT' && !(error instanceof SyntaxError)) throw error;
    }
  }
  let root = member;
  let workspaces;
  for (let dir = path.dirname(member); ; dir = path.dirname(dir)) {
    const pkg = readManifest(dir);
    if (pkg && pkg.workspaces) {
      const members = await mapWorkspaces({ cwd: dir, pkg });
      if ([...members.values()].includes(member)) {
        root = dir;
        workspaces = members;
        break;
      }
    }
    if (dir === path.dirname(dir)) break;
  }
  if (!workspaces) {
    const pkg = readManifest(member);
    if (!pkg || !pkg.workspaces) return null;
    workspaces = await mapWorkspaces({ cwd: member, pkg });
  }
  // An early npm 8 release or a nested node_modules can select the wrong
  // project configuration. Load the owner's npmrc before resolving selectors.
  const differentRoot = config.localPrefix !== root;
  if (differentRoot) {
    config = await loadConfig(root);
    if (nativeScope(config)) return null;
  }
  const selectors = config.find('workspace') === 'default' ? [] : config.get('workspace');
  const combined = selectors.length > 0 || config.get('workspaces') === true;
  const [command, ...operands] = config.parsedArgv.remain;
  if (command === 'install' && combined && config.get('save') && !config.get('dry-run')) {
    const npa = npm('npm-package-arg');
    const specs = operands.map((arg) => npa(arg, process.cwd()));
    const members = new Set(workspaces.values());
    const oldNpm = npm('semver').lt(npm('../package.json').version, '8.19.0');
    if (
      specs.some((spec) => {
        if (spec.type !== 'directory') return false;
        return oldNpm || spec.name || !members.has(spec.fetchSpec);
      })
    ) {
      throw new Error(
        'Add the local directory to the root and each selected workspace separately so npm saves the correct relative paths.',
      );
    }
    if (specs.some((spec) => spec.type === 'file' && spec.name)) {
      throw new Error(
        'Pass the tarball path without a package name, or add it to each target separately.',
      );
    }
    if (specs.some((spec) => spec.type === 'file') && oldNpm) {
      throw new Error(
        'Adding a local tarball to both the root and a workspace requires npm 8.19.0 or later.',
      );
    }
  }
  if (!differentRoot && (root === member || selectors.length > 0)) return null;
  // npm 8 resolves path selectors from its prefix. Preserve their origin
  // when selecting another prefix, including selectors configured in npmrc.
  return {
    root,
    selectors: selectors.map((value) =>
      /^(?:\.{1,2})(?:[\\/]|$)/.test(value) ? path.resolve(value) : value,
    ),
  };
}

module.exports = function enable(member, preload) {
  const original = Module.prototype.require;
  let active = true;
  function workspaceRequire(id) {
    const result = original.call(this, id);
    if (!active || id !== '@npmcli/config' || path.basename(this.filename) !== 'npm.js')
      return result;
    const npm = Module.createRequire(this.filename);
    if (
      path.basename(path.dirname(this.filename)) !== 'lib' ||
      !fs.existsSync(path.resolve(path.dirname(this.filename), '../package.json')) ||
      npm('../package.json').name !== 'npm'
    )
      return result;
    active = false;
    // A user preload may have wrapped this function after it was installed.
    if (Module.prototype.require === workspaceRequire) Module.prototype.require = original;
    // A lifecycle script can start another npm. Only the outer invocation is scoped.
    const options = (process.env.NODE_OPTIONS || '')
      .replace(preload + ' ', '')
      .replace(preload, '');
    if (options) process.env.NODE_OPTIONS = options;
    else delete process.env.NODE_OPTIONS;
    return class WorkspaceConfig extends result {
      constructor(options) {
        super(options);
        this.workspaceOptions = options;
      }
      async load() {
        const edit = await plan(result, this.workspaceOptions, npm, member);
        if (edit)
          this.loadLocalPrefix = () => {
            this.localPrefix = edit.root;
          };
        await super.load();
        if (edit && edit.selectors.length) this.set('workspace', edit.selectors);
      }
    };
  }
  Module.prototype.require = workspaceRequire;
};
