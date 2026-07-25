default:
    just --list

install:
    cargo install --path .

readme:
    quarto render README.qmd --to gfm
