#!/bin/bash
cargo build -j16 --release
hyperfine 'cargo run --release --bin openschafkopf -- suggest-card --rules "herz-solo von 0" "--hand" "hz su e8 e7" "--cards-on-table" "ea ez ek e9 ga gz gk g9 sa sz sk s9 eo go ho so ha" --simulate-hands all --branching "8,8" --prune none --verbose'
