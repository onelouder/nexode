import * as esbuild from 'esbuild';

const target = readTarget();
const watch = process.argv.includes('--watch');

async function main() {
  const buildOptions = target === 'webview' ? webviewBuildOptions() : extensionBuildOptions();

  if (watch) {
    const context = await esbuild.context(buildOptions);
    await context.watch();
    return;
  }

  await esbuild.build(buildOptions);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

function readTarget() {
  const value = readFlagValue('--target');
  if (value === 'extension' || value === 'webview') {
    return value;
  }

  return 'extension';
}

function readFlagValue(flag) {
  const index = process.argv.indexOf(flag);
  if (index < 0 || index + 1 >= process.argv.length) {
    return undefined;
  }

  return process.argv[index + 1];
}

function extensionBuildOptions() {
  return {
    entryPoints: ['src/extension.ts'],
    bundle: true,
    format: 'cjs',
    platform: 'node',
    target: 'node18',
    outfile: 'dist/extension.js',
    sourcemap: true,
    external: ['vscode'],
    logLevel: 'info',
  };
}

function webviewBuildOptions() {
  return {
    entryPoints: {
      'synapse-grid': 'webview/synapse-grid/index.tsx',
      kanban: 'webview/kanban/index.tsx',
    },
    bundle: true,
    format: 'iife',
    platform: 'browser',
    target: 'es2022',
    outdir: 'dist/webview',
    entryNames: '[name]',
    assetNames: 'assets/[name]-[hash]',
    minify: true,
    sourcemap: true,
    define: {
      'process.env.NODE_ENV': '"production"',
    },
    loader: {
      '.css': 'css',
      '.svg': 'dataurl',
    },
    jsx: 'automatic',
    logLevel: 'info',
  };
}
