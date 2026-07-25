# updatethis


A tiny CLI to programmatically update the version of an R package.

## Installation

``` sh
cargo install --git https://github.com/JosiahParry/updatethis --tag v0.1.0
```

## Usage

    Bump the version of an R package

    Usage: updatethis <COMMAND>

    Commands:
      version      Increment the Version field of a package's DESCRIPTION file
      current      Print the current version of a package
      set-version  Set the Version field to a specific version
      help         Print this message or the help of the given subcommand(s)

    Options:
      -h, --help     Print help
      -V, --version  Print version

### Increment the package version

    Increment the Version field of a package's DESCRIPTION file

    Usage: updatethis version <VERSION_TYPE> [PATH]

    Arguments:
      <VERSION_TYPE>  Which component of the version to bump [possible values: major, minor, patch, dev]
      [PATH]          Path to the package root (defaults to the current directory)

    Options:
      -h, --help  Print help

Made with 🤍 from the `ricochet` 🐇 team
