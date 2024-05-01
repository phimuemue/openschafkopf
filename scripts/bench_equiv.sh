#!/bin/bash
cargo build -j16 --release

hyperfine -r1 --show-output -- \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so' --hand 'gu ha hk h7 ez gz gk  SZ EU E7 H9 EA E8 SK  SA HU G7 E9 H8 HZ G8  S9 SU EK GA S8 S7 G9' --simulate-hands all --branching equiv4" \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so' --hand 'gu ha hk h7 ez gz gk  SZ EU E7 H9 EA E8 SK  SA HU G7 E9 H8 HZ G8  S9 SU EK GA S8 S7 G9' --simulate-hands all --branching equiv5" \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so' --hand 'gu ha hk h7 ez gz gk  SZ EU E7 H9 EA E8 SK  SA HU G7 E9 H8 HZ G8  S9 SU EK GA S8 S7 G9' --simulate-hands all --branching equiv6" \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so' --hand 'gu ha hk h7 ez gz gk  SZ EU E7 H9 EA E8 SK  SA HU G7 E9 H8 HZ G8  S9 SU EK GA S8 S7 G9' --simulate-hands all --branching equiv7" \
