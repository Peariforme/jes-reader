import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
} from 'vscode-languageclient/node';

/** Our language ID. */
const LANGUAGE = 'jes-sysout';

/**
 * Language IDs that IBM Z Open Editor assigns to spool files by name
 * (`*JESYSMSG*`, `*SYSPRINT*`, `*JESMSGLG*`, `*JESJCL*`). Z Open Editor has
 * no language server for them: when it is installed it keeps its
 * highlighting and we add diagnostics, the Outline and folding.
 */
const ZOPEN_LANGUAGES = ['spool', 'jesmsglg', 'jesjcl'];

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand('jesReader.openAsSysout', openAsSysout),
    vscode.commands.registerCommand('jesReader.restartServer', () => restart(context)),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration('jesReader')) {
        void restart(context);
      }
    }),
  );
  await start(context);
}

export async function deactivate(): Promise<void> {
  await client?.stop();
}

async function start(context: vscode.ExtensionContext): Promise<void> {
  const command = serverPath(context);
  if (!command) {
    void vscode.window.showErrorMessage(
      'JES Reader: jes-reader-lsp binary not found. Set "jesReader.serverPath" ' +
        'or run "npm run build:server".',
    );
    return;
  }
  const config = vscode.workspace.getConfiguration('jesReader');
  const serverOptions: ServerOptions = { command, args: [] };
  const clientOptions: LanguageClientOptions = {
    // No `scheme`: local files, untitled documents and documents opened
    // by Zowe Explorer (`zowe-jobs:`).
    documentSelector: [LANGUAGE, ...ZOPEN_LANGUAGES].map((language) => ({ language })),
    initializationOptions: {
      maxReturnCode: config.get<number>('maxReturnCode', 4),
    },
  };
  client = new LanguageClient('jesReader', 'JES Reader', serverOptions, clientOptions);
  await client.start();
}

async function restart(context: vscode.ExtensionContext): Promise<void> {
  await client?.stop();
  client = undefined;
  await start(context);
}

/** Binary chosen by the user, otherwise the one bundled in `server/`. */
function serverPath(context: vscode.ExtensionContext): string | undefined {
  const configured = vscode.workspace.getConfiguration('jesReader').get<string>('serverPath');
  if (configured) {
    return configured;
  }
  const exe = process.platform === 'win32' ? 'jes-reader-lsp.exe' : 'jes-reader-lsp';
  const bundled = context.asAbsolutePath(path.join('server', exe));
  return fs.existsSync(bundled) ? bundled : undefined;
}

/** Forces the language ID of the active document, whatever its name. */
async function openAsSysout(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (editor) {
    await vscode.languages.setTextDocumentLanguage(editor.document, LANGUAGE);
  }
}
