#!/bin/bash

./target/release/openschafkopf suggest-card --rules "2 spielt mit der Eichel-Sau" --hand "go gu hk h7  gk g7  sa s7" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache --constrain-hands "ctx.trumpf(2)>=4"

