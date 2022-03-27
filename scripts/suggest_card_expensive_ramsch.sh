#!/bin/bash

# to be run from the openschafkopf directory

cargo build --release -j16

hyperfine -r1 --show-output 'target/release/openschafkopf suggest-card --hand "h8 su eu ga eo gk" --cards-on-table "g9 g7 gz g8  e8 e9 ez ek" --simulate-hands "gu ha go s9 ea e7  ho h9 sz s7 sk s8  so hz hk sa h7 hu" --rules "ramsch" --branching equiv7'
