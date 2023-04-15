#!/bin/bash

./target/release/openschafkopf suggest-card --rules "3 spielt mit der Eichel-Ass" --hand "ho su h9 h8  sk s8" --cards-on-table "ek ha e9 e7  ga gz hk gk" --simulate-hands 20 --branching equiv6 --verbose --points --snapshotcache

echo "Now explicitly assume that 0 is Schelln-frei"

./target/release/openschafkopf suggest-card --rules "3 spielt mit der Eichel-Ass" --hand "ho su h9 h8  sk s8" --cards-on-table "ek ha e9 e7  ga gz hk gk" --simulate-hands 20 --branching equiv6 --points --snapshotcache --constrain-hands "ctx.schelln(0)==0"
