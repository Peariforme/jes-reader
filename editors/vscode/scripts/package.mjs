// Builds a platform-specific VSIX: compiles jes-reader-lsp for the target,
// puts it in server/ and runs `vsce package --target`.
//
//   node scripts/package.mjs [vscode-target]
//
// Without an argument, packages for the current machine. The VSIX lands in
// dist/jes-reader-<target>-<version>.vsix.
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

/** VS Code target → Rust target. Linux uses musl for a static binary. */
const TARGETS = {
  'win32-x64': 'x86_64-pc-windows-msvc',
  'win32-arm64': 'aarch64-pc-windows-msvc',
  'linux-x64': 'x86_64-unknown-linux-musl',
  'linux-arm64': 'aarch64-unknown-linux-musl',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
};

const extension = dirname(dirname(fileURLToPath(import.meta.url)));
const workspace = join(extension, '..', '..');
const targetDir = process.env.CARGO_TARGET_DIR ?? join(workspace, 'target');

const target = process.argv[2] ?? `${process.platform}-${process.arch}`;
const rustTarget = TARGETS[target];
if (!rustTarget) {
  console.error(`Unknown target "${target}". Expected one of: ${Object.keys(TARGETS).join(', ')}`);
  process.exit(1);
}
const exe = target.startsWith('win32') ? 'jes-reader-lsp.exe' : 'jes-reader-lsp';
const { version } = JSON.parse(readFileSync(join(extension, 'package.json'), 'utf8'));

// 1. Language server for the target platform.
execFileSync(
  'cargo',
  ['build', '--release', '--locked', '-p', 'jes-reader-lsp', '--target', rustTarget],
  { cwd: workspace, stdio: 'inherit' },
);

// 2. Only this platform's binary in server/.
try {
  rmSync(join(extension, 'server'), { recursive: true, force: true });
} catch (error) {
  if (error.code !== 'EPERM' && error.code !== 'EBUSY') throw error;
  console.error('server/ is locked: close the VS Code windows running this extension (F5), then retry.');
  process.exit(1);
}
mkdirSync(join(extension, 'server'));
copyFileSync(
  join(targetDir, rustTarget, 'release', exe),
  join(extension, 'server', exe),
);

// 3. vsce requires a LICENSE file next to package.json: dual MIT / Apache-2.0.
const license = (name) => readFileSync(join(workspace, name), 'utf8');
writeFileSync(
  join(extension, 'LICENSE'),
  'Licensed under either of the MIT license or the Apache License, Version 2.0, at your option.\n\n' +
    `${license('LICENSE-MIT')}\n\n${license('LICENSE-APACHE')}`,
);

// 4. Platform-specific VSIX (vsce runs `vscode:prepublish`, which compiles).
mkdirSync(join(extension, 'dist'), { recursive: true });
const out = join('dist', `jes-reader-${target}-${version}.vsix`);
execFileSync('npx', ['vsce', 'package', '--target', target, '--out', out], {
  cwd: extension,
  stdio: 'inherit',
  shell: process.platform === 'win32',
});
console.log(`${out} ready`);
