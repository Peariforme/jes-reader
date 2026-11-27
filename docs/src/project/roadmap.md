# Roadmap

## Scope

JES Reader reads JES2 sysouts on the workstation and explains them. It stays out of:

- connecting to z/OS: Zowe Explorer and the Zowe CLI already do that;
- interpreting dumps;
- reproducing IBM manuals: explanations are written in our own words, with links to IBM Docs.

## Done

Everything planned for the first versions is there:

- a parser that works one DD at a time, as Zowe Explorer opens spools, and cuts whole spools (BMC AMI DevX) into their DDs;
- diagnostics, Outline, step summaries, folding, links to the JCL and to data sets, hovers, colors that follow the analysis;
- house rules (`.jesreader.toml`);
- the VS Code extension, with Zowe Explorer and IBM Z Open Editor;
- the `jes-reader` command line (text, JSON, SARIF) and the language server for other editors;
- releases for five platforms, offline VSIX, VS Code Marketplace and crates.io.

See the [changelog](changelog.md) for the details of each version.

## Next

### A more reliable parser

Sysouts vary from site to site: installation exits change the step tables, export tools add or remove the carriage-control column, products add their own DDs and messages. The parser never fails, but it can miss a step or a message on a layout it has not seen.

- More layouts in the corpus: started tasks, long jobs, step tables written by other `IEFACTRT` exits, other export tools.
- In whole spools, the output of the steps split into its DDs, with their names.
- Tests on malformed and unexpected input, so that no file makes the parser stop or lose its place.

### A larger message catalog

Hovers and severities come from a catalog of about 70 messages and 20 abend codes. Next:

- more messages from the components jobs meet every day: data management (`IEC`, `IGD`, VSAM), sort products, Db2, CICS, IMS, Language Environment and the compilers;
- the reason codes of the common abends (`S0C4`, `S806`, `Sx37`, `S913`…), and what to check for each.

## How you can help

The most useful contribution is a message JES Reader does not know, or reports with the wrong severity: [open a "Message or abend" issue](https://github.com/Peariforme/jes-reader/issues/new?template=message.yml) with the message ID and one anonymized line. A layout the parser gets wrong is just as welcome as a [bug report](https://github.com/Peariforme/jes-reader/issues/new?template=bug.yml). Please never attach a real sysout from your site.
