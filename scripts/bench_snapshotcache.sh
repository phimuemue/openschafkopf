#!/bin/bash
cargo build -j16 --release --no-default-features --features suggest-card

hyperfine -r1 --show-output -- \
    "./target/release/openschafkopf suggest-card --rules 'Rufspiel mit der Gras-Sau von 3' --cards-on-table 'so ha eu h8' --hand 'H9 SK SZ G7 GA EA SU  H7 HZ HK HU SA GK EK  GO G8 S8 S9 S7 E9 EZ  EO E8 GZ G9 E7 GU HO' --branching oracle --simulate-hands all --snapshotcache --points" \
    "./target/release/openschafkopf suggest-card --rules 'Rufspiel mit der Gras-Sau von 3' --cards-on-table 'so ha eu h8' --hand 'H9 SK SZ G7 GA EA SU  H7 HZ HK HU SA GK EK  GO G8 S8 S9 S7 E9 EZ  EO E8 GZ G9 E7 GU HO' --branching oracle --simulate-hands all --snapshotcache" \
    "./target/release/openschafkopf suggest-card --rules 'Rufspiel mit der Gras-Sau von 3' --cards-on-table 'so ha eu h8' --hand 'H9 SK SZ G7 GA EA SU  H7 HZ HK HU SA GK EK  GO G8 S8 S9 S7 E9 EZ  EO E8 GZ G9 E7 GU HO' --branching oracle --simulate-hands all"

hyperfine -r1 --show-output -- \
    "./target/release/openschafkopf suggest-card --rules 'Rufspiel mit der Gras-Sau von 3' --cards-on-table '' --hand 'SO H9 SK SZ G7 GA EA SU  HA H7 HZ HK HU SA GK EK  EU GO G8 S8 S9 S7 E9 EZ  H8 EO E8 GZ G9 E7 GU HO' --branching oracle --simulate-hands all --snapshotcache --verbose --points" \
    "./target/release/openschafkopf suggest-card --rules 'Rufspiel mit der Gras-Sau von 3' --cards-on-table '' --hand 'SO H9 SK SZ G7 GA EA SU  HA H7 HZ HK HU SA GK EK  EU GO G8 S8 S9 S7 E9 EZ  H8 EO E8 GZ G9 E7 GU HO' --branching oracle --simulate-hands all --snapshotcache --verbose"
