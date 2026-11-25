import * as vscode from 'vscode';

/**
 * Data set names in a sysout opened from Zowe Explorer become links that
 * open the data set (or member) through Zowe Explorer's file system:
 * `zowe-ds:/<profile>/<DSN>` or `zowe-ds:/<profile>/<PDS>/<MEMBER>`, the
 * profile being the one of the job (`zowe-jobs:/<profile>/…`).
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

/** `zowe-ds:` URI of a data set or member for a profile. */
export function datasetUri(profile: string, reference: DatasetReference): vscode.Uri {
  const path = reference.member
    ? `/${profile}/${reference.dsn}/${reference.member}`
    : `/${profile}/${reference.dsn}`;
  return vscode.Uri.from({ scheme: 'zowe-ds', path });
}

/** Command run by the links: opens a `zowe-ds:` URI in an editor. */
export const OPEN_DATASET_COMMAND = 'jesReader.openDataset';

export async function openDataset(uri: string, name: string): Promise<void> {
  try {
    const document = await vscode.workspace.openTextDocument(vscode.Uri.parse(uri));
    await vscode.window.showTextDocument(document, { preview: true });
  } catch (error) {
    const directory =
      error instanceof vscode.FileSystemError && error.code === 'FileIsADirectory';
    void vscode.window.showWarningMessage(
      directory
        ? `JES Reader: ${name} is a partitioned data set. Open one of its members from Zowe Explorer.`
        : `JES Reader: could not open ${name} through Zowe Explorer (${error instanceof Error ? error.message : String(error)}).`,
    );
  }
}

/** Links on data set names, for documents opened from Zowe Explorer only. */
export class DatasetLinkProvider implements vscode.DocumentLinkProvider {
  provideDocumentLinks(document: vscode.TextDocument): vscode.DocumentLink[] {
    const profile = zoweProfile(document.uri);
    if (!profile) {
      return [];
    }
    const links: vscode.DocumentLink[] = [];
    for (let line = 0; line < document.lineCount; line++) {
      for (const reference of datasetReferences(document.lineAt(line).text)) {
        const uri = datasetUri(profile, reference).toString();
        const name = reference.member ? `${reference.dsn}(${reference.member})` : reference.dsn;
        const target = vscode.Uri.parse(
          `command:${OPEN_DATASET_COMMAND}?${encodeURIComponent(JSON.stringify([uri, name]))}`,
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
