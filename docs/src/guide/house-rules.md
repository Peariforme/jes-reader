# House rules

Every site has its conventions: a product message that is not about your jobs, a return code 8 that is expected for a given step. A `.jesreader.toml` file adapts the analysis to them: silence a message, tolerate a return code, or raise a message to an error.

## Where JES Reader looks for it

In VS Code, in this order:

1. the `jesReader.rulesFile` setting;
2. `.jesreader.toml` at the root of the workspace;
3. `.jesreader.toml` in your home directory.

The command line takes `--rules <file>`, or `.jesreader.toml` in the current directory, then in the home directory.

Changes are picked up as soon as you save the file. An invalid file is reported in a notification and ignored.

## Example

```toml
# Return code above which a step is reported (default 4).
max_return_code = 4

# Messages: severity "error", "warning", "info" or "ignore". `*` matches anything.
[[messages]]
id = "IDI0078E"
severity = "ignore"
note = "Fault Analyzer history file, not about the job"

[[messages]]
id = "IEF287I"
text = "NOT CATLGD 2"      # only when the message text contains this
severity = "error"

# Steps: return code tolerated, by step name (`step` or `step.procstep`, `*` allowed)…
[[steps]]
name = "COBRUN.LKED"
max_return_code = 8
note = "link-edit warnings are expected"

# …or by program (`PGM=` of the EXEC statement), or both.
[[steps]]
program = "IDCAMS"
max_return_code = 8
note = "RC 8: nothing to delete"
```

## Reference

### `max_return_code`

Return code above which a step is reported as a warning, for the steps no `[[steps]]` rule matches. When it is absent, the `jesReader.maxReturnCode` setting applies (default 4). On the command line, `--max-rc` overrides it.

### `[[messages]]`

| Key | Required | Meaning |
| --- | --- | --- |
| `id` | yes | Message ID, `*` matches any characters (`IGD*`) |
| `text` | no | The rule applies only when the message text contains this |
| `severity` | no | `error`, `warning`, `info` or `ignore`. Without it, the message keeps its severity and gets the note |
| `note` | no | Added to the diagnostic |

### `[[steps]]`

| Key | Required | Meaning |
| --- | --- | --- |
| `name` | one of `name`, `program` | Step name, `step` or `step.procstep`, `*` allowed |
| `program` | one of `name`, `program` | Program of the step (`PGM=` of its `EXEC`), `*` allowed |
| `max_return_code` | no | Highest return code accepted for that step. Without it, the file's `max_return_code` applies |
| `note` | no | Added to the diagnostic |

With both `name` and `program`, both must match.

## How rules apply

- Rules apply in file order: the first matching rule wins.
- Program rules need the job's `JESJCL`. JES Reader fetches it in the background for documents opened from Zowe Explorer, and reads `JESJCL.txt` next to downloaded files.
- Silenced messages are no longer colored as errors in the editor.

## Share them with your team

Commit `.jesreader.toml` at the root of the repository that holds your JCL: everyone who opens that folder in VS Code, and every CI run, applies the same rules.
