import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

interface Package {
  name: string;
  dependencies: string[];
  scripts: Record<string, string>;
  hasVitePlusConfig: boolean;
}

const __dirname = path.join(fileURLToPath(import.meta.url), '..');

class MonorepoGenerator {
  private packages: Map<string, Package> = new Map();
  private readonly PACKAGE_COUNT = 1000;
  private readonly MAX_DEPS_PER_PACKAGE = 8;
  private readonly MIN_DEPS_PER_PACKAGE = 2;
  private readonly SCRIPT_NAMES = [
    'build',
    'test',
    'lint',
    'dev',
    'start',
    'prepare',
    'compile',
  ];
  private readonly CATEGORIES = ['core', 'util', 'feature', 'service', 'app'];

  constructor(private rootDir: string) {}

  private getRandomInt(min: number, max: number): number {
    return Math.floor(Math.random() * (max - min + 1)) + min;
  }

  private getRandomElement<T>(arr: T[]): T {
    return arr[Math.floor(Math.random() * arr.length)];
  }

  private generatePackageName(index: number): string {
    const category = this.getRandomElement(this.CATEGORIES);
    const paddedIndex = index.toString().padStart(2, '0');
    return `${category}-${paddedIndex}`;
  }

  private generateScriptCommand(
    scriptName: string,
    packageName: string,
  ): string {
    const commands = [
      `echo "Running ${scriptName} for ${packageName}"`,
      `node scripts/${scriptName}.js`,
      `tsc --build`,
      `webpack build`,
      `rollup -c`,
      `esbuild src/index.js --bundle`,
      `npm run pre${scriptName}`,
      `node tasks/${scriptName}`,
    ];

    // Generate command with 0-3 && concatenations
    const numCommands = this.getRandomInt(1, 4);
    const selectedCommands: string[] = [];

    for (let i = 0; i < numCommands; i++) {
      selectedCommands.push(this.getRandomElement(commands));
    }

    return selectedCommands.join(' && ');
  }

  private generateScripts(packageName: string): Record<string, string> {
    const scripts: Record<string, string> = {};

    // Each package has 2-3 scripts
    const numScripts = this.getRandomInt(2, 3);
    const selectedScripts = new Set<string>();

    while (selectedScripts.size < numScripts) {
      selectedScripts.add(this.getRandomElement(this.SCRIPT_NAMES));
    }

    for (const scriptName of selectedScripts) {
      scripts[scriptName] = this.generateScriptCommand(scriptName, packageName);
    }

    return scripts;
  }

  private selectDependencies(
    currentIndex: number,
    availablePackages: string[],
  ): string[] {
    const numDeps = this.getRandomInt(
      this.MIN_DEPS_PER_PACKAGE,
      this.MAX_DEPS_PER_PACKAGE,
    );
    const dependencies = new Set<string>();

    // Create a complex graph by selecting dependencies from different layers
    // Prefer packages with lower indices (creates deeper dependency chains)
    const eligiblePackages = availablePackages.filter((pkg) => {
      const pkgIndex = parseInt(pkg.split('-')[1]);
      return pkgIndex < currentIndex;
    });

    if (eligiblePackages.length === 0) {
      return [];
    }

    while (
      dependencies.size < numDeps &&
      dependencies.size < eligiblePackages.length
    ) {
      const dep = this.getRandomElement(eligiblePackages);
      dependencies.add(dep);
    }

    // Add some cross-category dependencies for complexity
    if (Math.random() > 0.3) {
      const crossCategoryDeps = availablePackages.filter((pkg) => {
        const category = pkg.split('-')[0];
        return category !== currentIndex.toString().split('-')[0];
      });

      if (crossCategoryDeps.length > 0) {
        dependencies.add(this.getRandomElement(crossCategoryDeps));
      }
    }

    return Array.from(dependencies);
  }

  private generatePackages(): void {
    // First, create all package names
    const allPackageNames: string[] = [];
    for (let i = 0; i < this.PACKAGE_COUNT; i++) {
      allPackageNames.push(this.generatePackageName(i));
    }

    // Generate packages with dependencies
    for (let i = 0; i < this.PACKAGE_COUNT; i++) {
      const packageName = allPackageNames[i];
      const scripts = this.generateScripts(packageName);

      // 70% chance to have vite-plus.json config
      const hasVitePlusConfig = Math.random() > 0.3;

      // Select dependencies from packages created before this one
      const dependencies =
        i === 0 ? [] : this.selectDependencies(i, allPackageNames.slice(0, i));

      this.packages.set(packageName, {
        name: packageName,
        dependencies,
        scripts,
        hasVitePlusConfig,
      });
    }

    // Ensure complex transitive dependencies for script resolution testing
    this.addTransitiveScriptDependencies();
  }

  private addTransitiveScriptDependencies(): void {
    // Create specific patterns for testing transitive script dependencies
    const packagesArray = Array.from(this.packages.entries());

    for (let i = 0; i < 50; i++) {
      const [nameA, pkgA] = this.getRandomElement(packagesArray);
      const [nameB, pkgB] = this.getRandomElement(packagesArray);
      const [nameC, pkgC] = this.getRandomElement(packagesArray);

      if (nameA !== nameB && nameB !== nameC && nameA !== nameC) {
        // Setup: A depends on B, B depends on C
        if (!pkgA.dependencies.includes(nameB)) {
          pkgA.dependencies.push(nameB);
        }
        if (!pkgB.dependencies.includes(nameC)) {
          pkgB.dependencies.push(nameC);
        }

        // Create the scenario: A has build, B doesn't, C has build
        const scriptName = this.getRandomElement(this.SCRIPT_NAMES);
        pkgA.scripts[scriptName] = this.generateScriptCommand(
          scriptName,
          nameA,
        );
        delete pkgB.scripts[scriptName]; // B doesn't have the script
        pkgC.scripts[scriptName] = this.generateScriptCommand(
          scriptName,
          nameC,
        );
      }
    }
  }

  private writePackage(pkg: Package): void {
    const packageDir = path.join(this.rootDir, 'packages', pkg.name);

    // Create directory structure
    fs.mkdirSync(packageDir, { recursive: true });
    fs.mkdirSync(path.join(packageDir, 'src'), { recursive: true });

    // Write package.json
    const packageJson = {
      name: `@monorepo/${pkg.name}`,
      version: '1.0.0',
      main: 'src/index.js',
      scripts: pkg.scripts,
      dependencies: pkg.dependencies.reduce(
        (deps, dep) => {
          deps[`@monorepo/${dep}`] = 'workspace:*';
          return deps;
        },
        {} as Record<string, string>,
      ),
    };

    fs.writeFileSync(
      path.join(packageDir, 'package.json'),
      JSON.stringify(packageJson, null, 2),
    );

    // Write source file
    const indexContent = `// ${pkg.name} module
export function ${pkg.name.replace('-', '_')}() {
  console.log('Executing ${pkg.name}');
${pkg.dependencies.map((dep) => `  require('@monorepo/${dep}');`).join('\n')}
}

module.exports = { ${pkg.name.replace('-', '_')} };
`;

    fs.writeFileSync(path.join(packageDir, 'src', 'index.js'), indexContent);

    // Write vite-plus.json if needed
    if (pkg.hasVitePlusConfig) {
      const vitePlusConfig = {
        extends: '../../vite-plus.json',
        tasks: {
          build: {
            cache: true,
            env: {
              NODE_ENV: 'production',
            },
          },
        },
      };

      fs.writeFileSync(
        path.join(packageDir, 'vite-plus.json'),
        JSON.stringify(vitePlusConfig, null, 2),
      );
    }
  }

  public generate(): void {
    console.log('Generating monorepo structure...');

    // Clean and create root directory
    if (fs.existsSync(this.rootDir)) {
      fs.rmSync(this.rootDir, { recursive: true, force: true });
    }
    fs.mkdirSync(this.rootDir, { recursive: true });
    fs.mkdirSync(path.join(this.rootDir, 'packages'), { recursive: true });

    // Generate packages
    this.generatePackages();

    // Write all packages
    let count = 0;
    for (const [_, pkg] of this.packages) {
      this.writePackage(pkg);
      count++;
      if (count % 100 === 0) {
        console.log(`Generated ${count} packages...`);
      }
    }

    // Write root package.json
    const rootPackageJson = {
      name: 'monorepo-benchmark',
      version: '1.0.0',
      private: true,
      workspaces: ['packages/*'],
      scripts: {
        'build:all': 'vite run build',
        'test:all': 'vite run test',
        'lint:all': 'vite run lint',
      },
      devDependencies: {
        '@voidzero-dev/vite-plus': '*',
      },
    };

    fs.writeFileSync(
      path.join(this.rootDir, 'package.json'),
      JSON.stringify(rootPackageJson, null, 2),
    );

    // Write pnpm-workspace.yaml for pnpm support
    const pnpmWorkspace = `packages:
  - 'packages/*'
`;
    fs.writeFileSync(
      path.join(this.rootDir, 'pnpm-workspace.yaml'),
      pnpmWorkspace,
    );

    // Write root vite-plus.json
    const rootVitePlusConfig = {
      tasks: {
        build: {
          cache: true,
          parallel: true,
        },
        test: {
          cache: true,
          parallel: true,
        },
        lint: {
          cache: false,
          parallel: true,
        },
      },
    };

    fs.writeFileSync(
      path.join(this.rootDir, 'vite-plus.json'),
      JSON.stringify(rootVitePlusConfig, null, 2),
    );

    console.log(
      `Successfully generated monorepo with ${this.PACKAGE_COUNT} packages!`,
    );
    console.log(`Location: ${this.rootDir}`);

    // Print some statistics
    this.printStatistics();
  }

  private printStatistics(): void {
    let totalDeps = 0;
    let maxDeps = 0;
    let packagesWithVitePlus = 0;
    const scriptCounts = new Map<string, number>();

    for (const [_, pkg] of this.packages) {
      totalDeps += pkg.dependencies.length;
      maxDeps = Math.max(maxDeps, pkg.dependencies.length);

      if (pkg.hasVitePlusConfig) {
        packagesWithVitePlus++;
      }

      for (const script of Object.keys(pkg.scripts)) {
        scriptCounts.set(script, (scriptCounts.get(script) || 0) + 1);
      }
    }

    console.log('\nStatistics:');
    console.log(`- Total packages: ${this.packages.size}`);
    console.log(
      `- Average dependencies per package: ${(
        totalDeps / this.packages.size
      ).toFixed(2)}`,
    );
    console.log(`- Max dependencies in a package: ${maxDeps}`);
    console.log(`- Packages with vite-plus.json: ${packagesWithVitePlus}`);
    console.log('- Script distribution:');
    for (const [script, count] of scriptCounts) {
      console.log(`  - ${script}: ${count} packages`);
    }
  }
}

// Main execution
const outputDir = path.join(__dirname, 'fixtures', 'monorepo');
const generator = new MonorepoGenerator(outputDir);
generator.generate();
