#!/bin/bash

./target/release/openschafkopf suggest-card --rules "0 spielt mit der Eichel-Ass" --hand "ho hz  ga gz  sk s9 s8" --cards-on-table "eo h9 hu h7  gu hk so" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache --constrain-hands "ctx.trumpf(0)>=2"

echo "Assume that 1 is my partner"

./target/release/openschafkopf suggest-card --rules "0 spielt mit der Eichel-Ass" --hand "ho hz  ga gz  sk s9 s8" --cards-on-table "eo h9 hu h7  gu hk so" --simulate-hands 20 --branching oracle --points --snapshotcache --constrain-hands "ctx.trumpf(0)>=2 && ctx.ea(1)==0"

echo "Assume that 1 is my partner and HA is at 0"

./target/release/openschafkopf suggest-card --rules "0 spielt mit der Eichel-Ass" --hand "ho hz  ga gz  sk s9 s8" --cards-on-table "eo h9 hu h7  gu hk so" --simulate-hands 20 --branching oracle --points --snapshotcache --constrain-hands "ctx.trumpf(0)>=2 && ctx.ea(1)==0 && ctx.ha(0)==1"
