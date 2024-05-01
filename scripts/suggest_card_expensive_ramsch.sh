#!/bin/bash

# to be run from the openschafkopf directory

cargo build --release -j16

hyperfine -r1 --show-output 'target/release/openschafkopf suggest-card --hand "gu ha go s9 ea e7  h8 su eu ga eo gk  ho h9 sz s7 sk s8  so hz hk sa h7 hu" --cards-on-table "g9 g7 gz g8  e8 e9 ez ek" --rules "ramsch" --branching equiv7'
