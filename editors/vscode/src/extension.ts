import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
} from 'vscode-languageclient/node';

/** Notre language id. */
const LANGUAGE = 'jes-sysout';

/**
 * Language ids qu'IBM Z Open Editor attribue aux spools d'après leur nom
 * (`*JESYSMSG*`, `*SYSPRINT*`, `*JESMSGLG*`, `*JESJCL*`). Z Open Editor n'a
 * pas de serveur de langage pour eux : quand il est installé, il garde sa
 * coloration et nous ajoutons diagnostics, Outline et repliage.
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
      'JES Reader : binaire jes-reader-lsp introuvable. Renseigner « jesReader.serverPath » ' +
        'ou lancer « npm run build:server ».',
    );
    return;
  }
  const config = vscode.workspace.getConfiguration('jesReader');
  const serverOptions: ServerOptions = { command, args: [] };
  const clientOptions: LanguageClientOptions = {
    // Sans `scheme` : fichiers locaux, documents sans titre et documents
    // ouverts par Zowe Explorer (`zowe-jobs:`).
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

/** Binaire choisi par l'utilisateur, sinon binaire embarqué dans `server/`. */
function serverPath(context: vscode.ExtensionContext): string | undefined {
  const configured = vscode.workspace.getConfiguration('jesReader').get<string>('serverPath');
  if (configured) {
    return configured;
  }
  const exe = process.platform === 'win32' ? 'jes-reader-lsp.exe' : 'jes-reader-lsp';
  const bundled = context.asAbsolutePath(path.join('server', exe));
  return fs.existsSync(bundled) ? bundled : undefined;
}

/** Force le language id du document actif, quel que soit son nom. */
async function openAsSysout(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (editor) {
    await vscode.languages.setTextDocumentLanguage(editor.document, LANGUAGE);
  }
}
