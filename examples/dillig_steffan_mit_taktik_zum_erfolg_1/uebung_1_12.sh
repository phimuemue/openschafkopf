#!/bin/bash

./target/release/openschafkopf suggest-card --rules "1 spielt mit der Eichel-Sau" --hand "go ho eu h7 sk s7" --cards-on-table "ea ez e7 e8  gk hk g7 gz" --simulate-hands 100 --branching equiv6 --verbose --points --snapshotcache --constrain-hands "ctx.schelln(0)==0 || ctx.trumpf(0)==0"

echo ""

./target/release/openschafkopf suggest-card --rules "1 spielt mit der Eichel-Sau" --hand "go ho eu h7 sk s7" --cards-on-table "ea ez e7 e8  gk hk g7 gz" --simulate-hands 100 --branching equiv6 --points --snapshotcache

