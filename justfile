default:
    just --list

install:
    cargo install --path .

readme:
    just install
    quarto render README.qmd --to gfm

bump:
    #!/usr/bin/env bash
    version=$(git sv nv)
    if [[ -z "$version" ]]; then
        echo "No new version (git sv nv returned nothing)"
        exit 1
    fi
    just readme
    cargo release version $version --execute
    git add Cargo.toml Cargo.lock README.md
    git push
    git sv tag
    git push --tags
