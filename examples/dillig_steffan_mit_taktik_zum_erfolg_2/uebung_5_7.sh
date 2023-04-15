#!/bin/bash

./target/release/openschafkopf suggest-card --rules "0 spielt Herz-Wenz" --hand "eu ha h8  ek  gz gk  sk s7" --cards-on-table "gu" --simulate-hands 20 --branching oracle --points --snapshotcache --verbose --constrain-hands "ctx.trumpf(0)>=5 && (ctx.eichel(0)==0||ctx.gras(0)==0||ctx.schelln(0)==0)"

