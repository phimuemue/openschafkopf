#!/bin/bash

./target/release/openschafkopf suggest-card --rules "0 spielt Herz-Wenz" --hand "ea ek  gz gk g7  sz s9 s8" --cards-on-table "gu h8" --simulate-hands 20 --branching oracle --points --snapshotcache --verbose --constrain-hands "ctx.trumpf(0)>=5 && (ctx.eichel(0)==0||ctx.gras(0)==0||ctx.schelln(0)==0)"

