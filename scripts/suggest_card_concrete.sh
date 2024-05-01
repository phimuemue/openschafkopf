#!/bin/bash

# to be run from the openschafkopf directory

cargo build --release -j16

hyperfine -r1 --show-output -- "./target/release/openschafkopf suggest-card --cards-on-table 'e7 ea ez e8   go hk h8 h7   SA sk' --hand 'GU su G9 HA SZ S9   EK G8 HO S7 HZ   EO EU GZ G7 S8  so HU h9 ga gk e9' --rules 'eichel rufspiel von 2' --branching 'equiv7'"
hyperfine -r1  --show-output -- "./target/release/openschafkopf suggest-card --cards-on-table 'e7 ea ez e8  go hk h8 h7'  --hand 'so HU h9 ga gk e9  EO EU GZ G7 S8 sa  GU su G9 HA SZ S9  EK G8 HO S7 HZ sk' --rules 'eichel rufspiel von 2' --branching equiv7"
hyperfine -r1  --show-output -- "./target/release/openschafkopf suggest-card --cards-on-table 'e7 ea ez e8'  --hand 'GU su G9 HA SZ S9 h7  go so HU h9 ga gk e9  hk EK G8 HO S7 HZ sk  h8 EO EU GZ G7 S8 sa' --rules 'eichel rufspiel von 2' --branching equiv7"

