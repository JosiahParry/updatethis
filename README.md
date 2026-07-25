# updatethis


A tiny CLI to programmatically update the version of an R package.

## Installation

Install the latest prebuilt binary:

``` sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/JosiahParry/updatethis/releases/latest/download/updatethis-installer.sh | sh
```

On Windows:

``` powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/JosiahParry/updatethis/releases/latest/download/updatethis-installer.ps1 | iex"
```

Or build from source:

``` sh
cargo install --git https://github.com/JosiahParry/updatethis
```

## GitHub Actions

`updatethis` is designed to be used in a CI workflow so that version
management is easy.

To create the GitHub action:

``` sh
updatethis init
```

That creates `.github/workflows/set-version.yml`:

``` yaml
name: set-version

on:
  push:
    tags:
      - "v*.*.*"
      - "v*.*.*.*"

jobs:
  set-version:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v6
      - uses: JosiahParry/updatethis@v1
```

Create a tag for the version:

``` sh
git tag v0.2.0
git push --tags
```

The version must be `vMaj.Min.Patch{.dev}`.

> [!NOTE]
>
> Semantic versioning pre-release and build metadata are not supported.
> This is to adhere with the status quo set by CRAN.

The version from the tag is then written to the `DESCRIPTION` and
committed.

### Using Conventional Commits

It is recommended to use [conventional
commits](https://www.conventionalcommits.org) and
[git-sv](https://github.com/thegeeklab/git-sv) to set the version.

``` sh
git sv tag
git push --tags
```

### Action inputs

| Input | Default | Description |
|----|----|----|
| `version` | the tag that triggered the run | Version to set |
| `path` | `.` | Package root holding `DESCRIPTION` |
| `force` | `false` | Set the version even if it is not greater |
| `commit` | `true` | Commit the updated `DESCRIPTION` |
| `push` | `true` | Push the commit |
| `branch` | the default branch | Branch to push to |
| `commit-message` | `chore: set version to {{version}}` | Commit message |

## Usage

    Bump the version of an R package

    Usage: updatethis <COMMAND>

    Commands:
      version      Increment the Version field of a package's DESCRIPTION file
      current      Print the current version of a package
      set-version  Set the Version field to a specific version
      init         Write a GitHub Actions workflow that sets the version from git tags
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

The new version must be greater than the current one unless you pass
`--force`.

------------------------------------------------------------------------

Made with 🤍 from the `ricochet` 🐇 team
