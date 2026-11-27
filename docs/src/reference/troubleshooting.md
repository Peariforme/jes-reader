# Troubleshooting

## A file is not analysed

Check the language mode in the status bar, at the bottom right. If it is not **JES2 Sysout** (or `spool`, `jesmsglg`, `jesjcl` with IBM Z Open Editor), the file was not recognized: run **JES Reader: Open as Sysout**. See [Recognized files](../guide/recognized-files.md) for the names and contents JES Reader recognizes.

## No program in the step summaries, or program rules do not apply

Step programs come from `JESJCL`. For a downloaded job, `JESJCL.txt` must be next to the file (`JOBnnnnn/JES2/JESJCL.txt`). For a Zowe Explorer document, JES Reader fetches it in the background: see the next section if that fails.

## Zowe Explorer credentials

A data set link, or the `JESJCL` that JES Reader fetches in the background for a Zowe spool, goes through Zowe Explorer, which may ask for your credentials. If you cancel that prompt, Zowe Explorer may refuse every read from then on (`AuthCancelledError: User cancelled previous authentication`), even after you connect from its tree. Run **Developer: Reload Window** to start again.

To avoid it, connect from Zowe Explorer before following links.

## Slowness

The **JES Reader** output channel (View → Output) logs:

- the start of the language server;
- the analysis of each document: lines, DDs found with their lines and column offset, diagnostics, time;
- requests slower than 20 ms;
- the data set links computed for each document;
- the `JESJCL` fetched in the background.

Set its level to Trace (gear icon of the Output view) to see more. Set `jesReader.trace.server` to `messages` to log every exchange with the language server and its duration.

All extensions share one extension host: when links or hovers come late although JES Reader's log shows short times, another extension is busy. **Developer: Show Running Extensions** and **Developer: Start Extension Host Profile** show which one.

## A message is reported although it is not about my job

Some product messages are reported by their suffix even when they are not about your job, for example Fault Analyzer's `IDI0078E`. Silence them with a [house rule](../guide/house-rules.md):

```toml
[[messages]]
id = "IDI0078E"
severity = "ignore"
```

## Known limitations

- JES2 only: JES3 sysouts are not recognized.
- In a whole spool, the output of the steps (`SYSPRINT`, `SYSOUT`…) is one block: its DDs are not separated or named yet.
- A step that runs a program loaded by a previous step (`PGM=*.LKED.SYSLMOD`) shows that reference as its program.
- Elapsed time per step is known to the minute only in `JESYSMSG`.

## Reporting a problem

Open an [issue on GitHub](https://github.com/Peariforme/jes-reader/issues/new/choose). **Never attach a real sysout from your site**: it may hold data set names, user IDs and business data. Describe the message pattern instead, or rebuild a minimal example with the names changed.
