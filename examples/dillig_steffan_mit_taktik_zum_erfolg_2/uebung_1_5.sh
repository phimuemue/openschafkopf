#!/bin/bash

./target/release/openschafkopf suggest-card --rules "2 spielt mit der Eichel-Ass" --hand "go so gu hk h7  ek  sa s7" --cards-on-table "g8 g9" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache
