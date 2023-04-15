#!/bin/bash

./target/release/openschafkopf suggest-card --rules "1 spielt mit der Eichel-Sau" --hand "eo so gu h9  ez e9 e7  sz" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache --constrain-hands "ctx.trumpf(1)>=4"

