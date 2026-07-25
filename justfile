default:
    just --list

install:
    cargo install --path .

readme:
    quarto render README.qmd --to gfm

bump:
    #!/usr/bin/env bash
    version=$(git sv nv)
    if [[ -z "$version" ]]; then
        echo "No new version (git sv nv returned nothing)"
        exit 1
    fi
    cargo release version $version --execute
    git add Cargo.toml
    git sv tag
    git push && git push --tags
