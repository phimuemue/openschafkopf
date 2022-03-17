#!/bin/bash

# to be run from the openschafkopf directory

find main -name *.rs -exec touch '{}' \;
hyperfine -r1 -- "cargo build --release -j16"

