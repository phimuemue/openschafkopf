#!/bin/bash

./target/release/openschafkopf suggest-card --rules "1 spielt mit der Eichel-Sau" --hand "hu h8  ea ek e8  gz g8  sk" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache --constrain-hands "ctx.trumpf(1)>=4 && (ctx.trumpf(3)>=4 && ctx.eichel(3)==0)"

