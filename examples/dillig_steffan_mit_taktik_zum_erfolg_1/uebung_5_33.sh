#!/bin/bash

./target/release/openschafkopf suggest-card --rules "Herz-Solo von 2" --cards-on-table "ea ez e9 e7  ek sz ho" --hand "eo eu hk gk g7 sk s9" --simulate-hands 10 --branching equiv5 --verbose --constrain-hands "ctx.trumpf(2)>5" --points --snapshotcache
