#!/bin/bash
cargo build -j16 --release --no-default-features --features suggest-card

hyperfine -r3 -- \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so  sz sa s9 ha   h7 eu hz su  sk e9 S7 gk' --hand 'E7 H9 EA E8' --simulate-hands all --branching equiv6" \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so  sz sa s9 ha   h7 eu hz su  sk e9 S7 gk' --hand 'E7 H9 EA E8' --simulate-hands all --branching oracle"

hyperfine -r3 -- \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so  sz sa s9 ha   h7 eu hz su' --hand '__ __ ez gz gk  E7 H9 EA E8 SK  __ __ E9 H8 G8  __ __ S8 S7 G9' --simulate-hands all --branching equiv6" \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so  sz sa s9 ha   h7 eu hz su' --hand '__ __ ez gz gk  E7 H9 EA E8 SK  __ __ E9 H8 G8  __ __ S8 S7 G9' --simulate-hands all --branching oracle"

hyperfine -r1 -- \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so  sz sa s9' --hand 'gu ha hk h7 ez gz gk  EU E7 H9 EA E8 SK  HU G7 E9 H8 HZ G8  SU EK GA S8 S7 G9' --simulate-hands all --branching equiv6" \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so  sz sa s9' --hand 'gu ha hk h7 ez gz gk  EU E7 H9 EA E8 SK  HU G7 E9 H8 HZ G8  SU EK GA S8 S7 G9' --simulate-hands all --branching oracle"

hyperfine -r1 --show-output -- \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so' --hand 'gu ha hk h7 ez gz gk  SZ EU E7 H9 EA E8 SK  SA HU G7 E9 H8 HZ G8  S9 SU EK GA S8 S7 G9' --simulate-hands all --branching equiv6" \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so' --hand 'gu ha hk h7 ez gz gk  SZ EU E7 H9 EA E8 SK  SA HU G7 E9 H8 HZ G8  S9 SU EK GA S8 S7 G9' --simulate-hands all --branching oracle" \
    "./target/release/openschafkopf suggest-card --rules 'rufspiel eichel von 0' --cards-on-table 'go eo ho so' --hand 'gu ha hk h7 ez gz gk  SZ EU E7 H9 EA E8 SK  SA HU G7 E9 H8 HZ G8  S9 SU EK GA S8 S7 G9' --simulate-hands all --branching oracle --snapshotcache"
