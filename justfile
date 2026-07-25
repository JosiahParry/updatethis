default:
    just --list

install:
    cargo install --path .

readme:
    just install
    quarto render README.qmd --to gfm

bump:
    #!/usr/bin/env bash
    set -euo pipefail

    version=$(git sv nv)
    if [[ -z "$version" ]]; then
        echo "No new version (git sv nv returned nothing)"
        exit 1
    fi

    cargo release version "$version" --execute --no-confirm
    just readme

    git add Cargo.toml Cargo.lock README.md README.qmd
    git commit -m "chore: release v$version"
    git push

    # tags the commit we just pushed, so the tagged tree has the new version
    git sv tag
    git push --tags
