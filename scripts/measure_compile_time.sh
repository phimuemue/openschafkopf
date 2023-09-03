#!/bin/bash

# to be run from the openschafkopf directory

find lib -name *.rs -exec touch '{}' \;
find main -name *.rs -exec touch '{}' \;
hyperfine -r1 -- "cargo build -j16"

