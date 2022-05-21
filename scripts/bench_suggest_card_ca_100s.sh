#!/bin/bash
cargo build -j16 --release
time cargo run -j16 --bin openschafkopf --release --  suggest-card --rules "herz-solo von 0" "--hand" "ha hz su e7" "--cards-on-table" "ea ez ek e9 ga gz gk g9 sa sz sk s9 eo go ho so" --simulate-hands all --branching "8,8" --prune none --verbose
