#!/bin/bash

# to be run from the openschafkopf directory

for subcommand in cli suggest-card parse analyze websocket hand-stats dl webext; do
    echo $subcommand
    hyperfine -r5 --prepare "find main -name *.rs -exec touch '{}' \;" -- "cargo build -j16 --no-default-features --features $subcommand"
done

hyperfine -r5 --prepare "find main -name *.rs -exec touch '{}' \;" -- "cargo build -j16"
