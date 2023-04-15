#!/bin/bash

./target/release/openschafkopf suggest-card --rules "2 spielt mit der Eichel-Ass" --hand "go ho hk h7  ea ez e8 e7" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache

echo "Now explicitly assume that 2 has Eichel-Ober"

./target/release/openschafkopf suggest-card --rules "2 spielt mit der Eichel-Ass" --hand "go ho hk h7  ea ez e8 e7" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache --constrain-hands "ctx.eo(2)==1"
