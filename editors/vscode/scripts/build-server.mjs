// Builds jes-reader-lsp in release mode and copies it into server/, where
// the extension looks for it by default.
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const extension = dirname(dirname(fileURLToPath(import.meta.url)));
const workspace = join(extension, '..', '..');
const exe = process.platform === 'win32' ? 'jes-reader-lsp.exe' : 'jes-reader-lsp';

execFileSync('cargo', ['build', '--release', '-p', 'jes-reader-lsp'], {
  cwd: workspace,
  stdio: 'inherit',
});
mkdirSync(join(extension, 'server'), { recursive: true });
copyFileSync(join(workspace, 'target', 'release', exe), join(extension, 'server', exe));
console.log(`server/${exe} ready`);
