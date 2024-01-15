# Prefix-by-date

Prefix files by date

This project started as a shell script that I rewrote to practice developping
a project from scratch in Rust, and I added features to it over time to test
different things and add functionalities.

```
Prefix files by date

Usage: prefix-by-date [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  Paths to process

Options:
  -v, --verbose...                 Increase logging verbosity
  -q, --quiet...                   Decrease logging verbosity
  -C, --config <DIR>               Sets a custom config directory
      --today                      Prefix by today's date
      --time                       Prefix by date and time
      --no-time                    Only prefix by date
  -i, --interactive <INTERACTIVE>  Start the program interactively or not [default: off] [possible values: off, text, gui]
  -m, --metadata <METADATA>        Metadata matchers to enable [possible values: none, created, modified, both]
  -h, --help                       Print help (see more with '--help')
  -V, --version                    Print version
```
