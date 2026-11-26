import * as vscode from 'vscode';

/**
 * Data set names in a sysout become links that open the data set (or
 * member) through Zowe Explorer's file system: `zowe-ds:/<profile>/<DSN>`
 * or `zowe-ds:/<profile>/<PDS>/<MEMBER>`. The profile is the one of the job
 * for documents opened from Zowe Explorer (`zowe-jobs:/<profile>/…`), and
 * the `jesReader.zoweProfile` setting for the others (BMC AMI DevX,
 * downloaded files). A PDS opens in Zowe Explorer's Data Sets view.
 */

/**
 * Qualified name with an optional member: `SYS1.MACLIB(IHAPSA)`. Not after
 * `/` (`//COBOL.SYSIN DD`, a DD override) or `*` (the character column of
 * a hex dump), not before `=` (`TIME.GO=`, an EXEC parameter override) or
 * `&`/`*` (dump text).
 */
const DATASET =
  /(?<![A-Z0-9#$@&.\-/*])([A-Z#$@][A-Z0-9#$@-]{0,7}(?:\.[A-Z#$@][A-Z0-9#$@-]{0,7})+)(?:\(([A-Z#$@][A-Z0-9#$@]{0,7})\))?(?![A-Z0-9#$@.(=&*])/g;

export interface DatasetReference {
  /** Data set name. */
  dsn: string;
  /** Member name, for `PDS(MEMBER)`. */
  member?: string;
  /** Offset of the reference in the line. */
  start: number;
  /** Offset after the reference. */
  end: number;
}

/**
 * Data set names in a line, without the names that cannot be opened:
 * temporary data sets (`SYS26329.T044304.RA000…`), JES2 spool data sets
 * (`….JOB00869.D0000101.?`) and names longer than 44 characters.
 */
export function datasetReferences(line: string): DatasetReference[] {
  const references: DatasetReference[] = [];
  for (const match of line.matchAll(DATASET)) {
    const dsn = match[1];
    const qualifiers = dsn.split('.');
    const temporary = /^SYS\d{5}$/.test(qualifiers[0]) && /^T\d{6}$/.test(qualifiers[1] ?? '');
    const spool = qualifiers.some((q) => /^(JOB|TSU|STC)\d{5}$/.test(q));
    if (dsn.length > 44 || temporary || spool) {
      continue;
    }
    const start = match.index ?? 0;
    references.push({ dsn, member: match[2], start, end: start + match[0].length });
  }
  return references;
}

/** Zowe profile of a document opened from Zowe Explorer's Jobs view. */
export function zoweProfile(uri: vscode.Uri): string | undefined {
  if (uri.scheme !== 'zowe-jobs') {
    return undefined;
  }
  return uri.path.split('/').filter(Boolean)[0];
}

/** Profile for the links of a document: its job's, otherwise the setting. */
function linkProfile(uri: vscode.Uri): string | undefined {
  const configured = vscode.workspace
    .getConfiguration('jesReader')
    .get<string>('zoweProfile', '')
    .trim();
  return zoweProfile(uri) ?? (configured || undefined);
}

/** `zowe-ds:` URI of a data set or member for a profile. */
export function datasetUri(profile: string, reference: DatasetReference): vscode.Uri {
  const path = reference.member
    ? `/${profile}/${reference.dsn}/${reference.member}`
    : `/${profile}/${reference.dsn}`;
  return vscode.Uri.from({ scheme: 'zowe-ds', path });
}

/** Command run by the links: opens a `zowe-ds:` URI in an editor. */
export const OPEN_DATASET_COMMAND = 'jesReader.openDataset';

export async function openDataset(
  uri: string,
  name: string,
  profile?: string,
  dsn?: string,
): Promise<void> {
  const target = vscode.Uri.parse(uri);
  // A PDS is a directory in Zowe Explorer's file system: ask first.
  if (profile && dsn && (await isDirectory(target))) {
    await openPartitioned(target, profile, dsn);
    return;
  }
  try {
    const document = await vscode.workspace.openTextDocument(target);
    await vscode.window.showTextDocument(document, { preview: true });
  } catch (error) {
    if (profile && dsn && isDirectoryError(error)) {
      await openPartitioned(target, profile, dsn);
      return;
    }
    void vscode.window.showWarningMessage(
      `JES Reader: could not open ${name} through Zowe Explorer (${error instanceof Error ? error.message : String(error)}).`,
    );
  }
}

/** True if Zowe Explorer's file system says `uri` is a directory (a PDS). */
async function isDirectory(uri: vscode.Uri): Promise<boolean> {
  try {
    const stat = await vscode.workspace.fs.stat(uri);
    return (stat.type & vscode.FileType.Directory) !== 0;
  } catch {
    return false;
  }
}

/**
 * Opening a PDS as a document fails, but not always with the
 * `FileIsADirectory` code: VS Code may wrap it in a plain error whose
 * message says "… that is actually a directory".
 */
export function isDirectoryError(error: unknown): boolean {
  if (error instanceof vscode.FileSystemError && error.code === 'FileIsADirectory') {
    return true;
  }
  const message = error instanceof Error ? error.message : String(error);
  return /actually a directory|FileIsADirectory|EISDIR/i.test(message);
}

/**
 * A PDS cannot be opened in an editor: show it in Zowe Explorer's Data Sets
 * view, filtered on its name (`zowe.ds.setDataSetFilter`, profile and
 * pattern). If Zowe Explorer does not have that command, offer its members
 * in a quick pick instead.
 */
async function openPartitioned(uri: vscode.Uri, profile: string, dsn: string): Promise<void> {
  try {
    await vscode.commands.executeCommand('zowe.ds.setDataSetFilter', profile, dsn);
    return;
  } catch {
    // Older or newer Zowe Explorer without this command: fall back below.
  }
  try {
    const entries = await vscode.workspace.fs.readDirectory(uri);
    const members = entries.map(([member]) => member).sort();
    const member = await vscode.window.showQuickPick(members, {
      title: `${dsn}: ${members.length} member(s)`,
      placeHolder: 'Member to open',
    });
    if (member) {
      const document = await vscode.workspace.openTextDocument(
        vscode.Uri.joinPath(uri, member),
      );
      await vscode.window.showTextDocument(document, { preview: true });
    }
  } catch (error) {
    void vscode.window.showWarningMessage(
      `JES Reader: could not list the members of ${dsn} (${error instanceof Error ? error.message : String(error)}).`,
    );
  }
}

/**
 * Links on data set names: in documents opened from Zowe Explorer, and in
 * the others when the `jesReader.zoweProfile` setting names a profile.
 */
export class DatasetLinkProvider implements vscode.DocumentLinkProvider {
  provideDocumentLinks(document: vscode.TextDocument): vscode.DocumentLink[] {
    const profile = linkProfile(document.uri);
    if (!profile) {
      return [];
    }
    const links: vscode.DocumentLink[] = [];
    for (let line = 0; line < document.lineCount; line++) {
      for (const reference of datasetReferences(document.lineAt(line).text)) {
        const uri = datasetUri(profile, reference).toString();
        const name = reference.member ? `${reference.dsn}(${reference.member})` : reference.dsn;
        const target = vscode.Uri.parse(
          `command:${OPEN_DATASET_COMMAND}?${encodeURIComponent(
            JSON.stringify([uri, name, profile, reference.dsn]),
          )}`,
        );
        const link = new vscode.DocumentLink(
          new vscode.Range(line, reference.start, line, reference.end),
          target,
        );
        link.tooltip = `Open ${name} with Zowe Explorer`;
        links.push(link);
      }
    }
    return links;
  }
}
