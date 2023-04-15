#!/bin/bash

./target/release/openschafkopf suggest-card --rules "2 spielt mit der Eichel-Sau" --hand "so hz ea ek e8 gz gk g7" --cards-on-table "s7 sk sa" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache --constrain-hands "ctx.eichel(0)==0"

