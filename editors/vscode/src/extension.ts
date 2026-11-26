import * as fs from 'node:fs';
import * as os from 'node:os';
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
    vscode.workspace.onDidOpenTextDocument((document) => {
      void detectSysout(document);
      void openSiblingJcl(document);
    }),
    vscode.commands.registerCommand(OPEN_DATASET_COMMAND, openDataset),
    vscode.languages.registerDocumentLinkProvider(
      [LANGUAGE, ...ZOPEN_LANGUAGES].map((language) => ({ language })),
      new DatasetLinkProvider(),
    ),
  );
  await start(context);
  for (const document of vscode.workspace.textDocuments) {
    void detectSysout(document);
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
      rulesFile: config.get<string>('rulesFile', ''),
    },
    // The server reloads the house rules when a rules file changes: in the
    // workspace, in the home directory (Zowe documents often have no
    // workspace), or the file chosen in the settings.
    synchronize: {
      fileEvents: rulesWatchers(config.get<string>('rulesFile', '')),
    },
    middleware: {
      // The JESJCL opened in the background may have been released by VS
      // Code since: Go to Definition, an explicit action, may fetch it
      // again. Hovers and code lenses do not: each fetch can make Zowe
      // Explorer ask for credentials.
      provideDefinition: async (document, position, token, next) => {
        await openSiblingJcl(document, true);
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

/** Watchers for the house rules files the server may read. */
function rulesWatchers(rulesFile: string): vscode.FileSystemWatcher[] {
  const watchers = [vscode.workspace.createFileSystemWatcher('**/.jesreader.toml')];
  const home = os.homedir();
  if (home) {
    watchers.push(
      vscode.workspace.createFileSystemWatcher(
        new vscode.RelativePattern(vscode.Uri.file(home), '.jesreader.toml'),
      ),
    );
  }
  if (rulesFile) {
    watchers.push(
      vscode.workspace.createFileSystemWatcher(
        new vscode.RelativePattern(
          vscode.Uri.file(path.dirname(rulesFile)),
          path.basename(rulesFile),
        ),
      ),
    );
  }
  return watchers;
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
async function openSiblingJcl(
  document: vscode.TextDocument,
  userAction = false,
): Promise<void> {
  const jcl = siblingJcl(document.uri);
  if (!jcl) {
    return;
  }
  const key = jcl.toString();
  if (vscode.workspace.textDocuments.some((open) => open.uri.toString() === key)) {
    return;
  }
  // In the background, try each JESJCL once per session: a fetch that
  // failed (spool purged, Zowe not connected) is not repeated, so that
  // JES Reader does not make Zowe Explorer ask for credentials again and
  // again. Go to Definition tries again.
  if (!userAction && jclAttempts.has(key)) {
    return;
  }
  jclAttempts.add(key);
  try {
    await vscode.workspace.openTextDocument(jcl);
  } catch {
    // Links are simply unavailable.
  }
}

/** JESJCL documents already fetched, or tried, in the background. */
const jclAttempts = new Set<string>();

/**
 * First line of a JES2 system DD, with or without the ASA column: JESMSGLG
 * banner, JESYSMSG header, JESJCL JOB card. Same rule as `firstLine` in
 * package.json.
 */
const JES2_FIRST_LINE =
  /^(1?\s*J E S 2 {2}J O B {2}L O G|\s*STMT NO\. MESSAGE|\s{7,9}1 \/\/\S+\s+JOB\b)/;

/**
 * Plain text documents that are sysouts get our language. VS Code applies
 * `firstLine` only when the file name gives no language, so a spool saved
 * as `.txt` (BMC AMI DevX) would otherwise stay plain text.
 */
async function detectSysout(document: vscode.TextDocument): Promise<void> {
  if (document.languageId !== 'plaintext') {
    return;
  }
  for (let line = 0; line < Math.min(document.lineCount, 5); line++) {
    const text = document.lineAt(line).text;
    if (text.trim() === '') {
      continue;
    }
    if (JES2_FIRST_LINE.test(text)) {
      await vscode.languages.setTextDocumentLanguage(document, LANGUAGE);
    }
    return;
  }
}

/** Forces the language ID of the active document, whatever its name. */
async function openAsSysout(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (editor) {
    await vscode.languages.setTextDocumentLanguage(editor.document, LANGUAGE);
  }
}
