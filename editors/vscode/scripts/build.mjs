// Bundles the extension client into a single out/extension.js with esbuild:
// the VSIX then ships one JavaScript file instead of node_modules.
//
//   node scripts/build.mjs               development build, with source map
//   node scripts/build.mjs --production  minified, no source map
//   node scripts/build.mjs --watch       rebuild on change
//
// Type checking is done separately by `tsc --noEmit` (npm run typecheck).
import * as esbuild from 'esbuild';

const production = process.argv.includes('--production');
const watch = process.argv.includes('--watch');

const options = {
  entryPoints: ['src/extension.ts'],
  bundle: true,
  outfile: 'out/extension.js',
  // Provided by VS Code at run time.
  external: ['vscode'],
  format: 'cjs',
  platform: 'node',
  // VS Code 1.90 (the minimum engine) runs extensions on Node.js 20.
  target: 'node20',
  minify: production,
  sourcemap: !production,
  sourcesContent: false,
  logLevel: 'info',
};

if (watch) {
  const context = await esbuild.context(options);
  await context.watch();
} else {
  await esbuild.build(options);
}
