#!/bin/bash

./target/release/openschafkopf suggest-card --rules "3 spielt Herz-Solo" --hand "gu  ga gz g9 g7  ez e7  sk" --cards-on-table "gk" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache --constrain-hands "ctx.trumpf(3)>=6 && {let an_farbe = [ctx.eichel(3), ctx.gras(3), ctx.schelln(3)]; an_farbe.sort(); an_farbe[0]==0 && an_farbe[1]==0}"

