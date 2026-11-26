import * as vscode from 'vscode';

/**
 * The "JES Reader" output channel, with timestamps and levels: the
 * extension and the language server write to it (the server through
 * `window/logMessage`). Trace lines show with the channel's level set to
 * Trace (Output view: the gear icon, or "Developer: Set Log Level").
 */
export const log = vscode.window.createOutputChannel('JES Reader', { log: true });

/** Name of a document for the log: the last segment of its path. */
export function documentName(uri: vscode.Uri): string {
  return uri.path.split('/').filter(Boolean).pop() ?? uri.toString();
}

/** Milliseconds since `start` (`performance.now()`), for the log. */
export function since(start: number): string {
  return `${(performance.now() - start).toFixed(1)} ms`;
}
