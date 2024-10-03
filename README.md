# Prefix-by-date

![workflow](https://github.com/Aethelflaed/prefix-by-date/actions/workflows/rust.yml/badge.svg?branch=main)

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

## Examples

![image](https://github.com/user-attachments/assets/5ca2175a-74a4-406a-ac2b-37b796128cf1)

![image](https://github.com/user-attachments/assets/10d36046-a01f-4dad-90c5-d56ec6aabe91)

![image](https://github.com/user-attachments/assets/9636699d-69fb-4a01-a318-9e052a3658b7)

![image](https://github.com/user-attachments/assets/5a761d31-844f-4a6f-a4a2-0af15b13ecdb)
