import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
} from 'vscode-languageclient/node';
import { DatasetLinkProvider, OPEN_DATASET_COMMAND, openDataset } from './datasets';

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
    vscode.workspace.onDidOpenTextDocument((document) => void openSiblingJcl(document)),
    vscode.commands.registerCommand(OPEN_DATASET_COMMAND, openDataset),
    vscode.languages.registerDocumentLinkProvider(
      [LANGUAGE, ...ZOPEN_LANGUAGES].map((language) => ({ language })),
      new DatasetLinkProvider(),
    ),
  );
  await start(context);
  for (const document of vscode.workspace.textDocuments) {
    void openSiblingJcl(document);
  }
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
    middleware: {
      // The JESJCL opened in the background may have been released by VS
      // Code since: make sure the server has it before resolving a link.
      provideDefinition: async (document, position, token, next) => {
        await openSiblingJcl(document);
        return next(document, position, token);
      },
      // The hover shows the JCL statement a link leads to: same need.
      provideHover: async (document, position, token, next) => {
        await openSiblingJcl(document);
        return next(document, position, token);
      },
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

/**
 * JESJCL of the same job, for a JESYSMSG or JESMSGLG document opened by Zowe
 * Explorer (`<job>.<jobid>.JES2.JESYSMSG.4` → `<job>.<jobid>.JES2.JESJCL.3`).
 * JES2 numbers its system DDs JESMSGLG 2, JESJCL 3, JESYSMSG 4. Downloaded
 * files need nothing: the server reads `JESJCL.txt` next to them.
 */
function siblingJcl(uri: vscode.Uri): vscode.Uri | undefined {
  if (uri.scheme === 'file' || uri.scheme === 'untitled') {
    return undefined;
  }
  const match = /\.JES2\.(?:JESYSMSG|JESMSGLG)\.\d+$/.exec(uri.path);
  if (!match) {
    return undefined;
  }
  return uri.with({ path: `${uri.path.slice(0, match.index)}.JES2.JESJCL.3` });
}

/**
 * Opens the sibling JESJCL in the background, without an editor tab: Zowe
 * fetches only that DD, and the language server receives it like any open
 * document, which enables Go to Definition from JESYSMSG and JESMSGLG.
 */
async function openSiblingJcl(document: vscode.TextDocument): Promise<void> {
  const jcl = siblingJcl(document.uri);
  if (!jcl) {
    return;
  }
  try {
    await vscode.workspace.openTextDocument(jcl);
  } catch {
    // Spool purged, DD not numbered as expected, Zowe not connected: links
    // are simply unavailable.
  }
}

/** Forces the language ID of the active document, whatever its name. */
async function openAsSysout(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (editor) {
    await vscode.languages.setTextDocumentLanguage(editor.document, LANGUAGE);
  }
}
