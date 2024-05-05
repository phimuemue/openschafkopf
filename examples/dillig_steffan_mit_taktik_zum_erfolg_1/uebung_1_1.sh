#!/bin/bash

./target/release/openschafkopf suggest-card --rules "0 spielt mit der Schell" --rules "0 spielt mit der Gras" --hand "eo so su hz h7  gz gk  s7" --simulate-hands 20 --branching oracle --points --snapshotcache
