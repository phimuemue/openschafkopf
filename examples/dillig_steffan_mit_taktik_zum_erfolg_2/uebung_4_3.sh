#!/bin/bash

./target/release/openschafkopf suggest-card --rules "3 spielt Eichel-Wenz" --hand "eu gu  ha hz h7  ea ek e7" --simulate-hands 20 --branching oracle --points --snapshotcache --verbose --position 3 --no-details
./target/release/openschafkopf suggest-card --rules "3 spielt Herz-Wenz" --hand "eu gu  ha hz h7  ea ek e7" --simulate-hands 20 --branching oracle --points --snapshotcache --position 3 --no-details
./target/release/openschafkopf suggest-card --rules "3 spielt Wenz" --hand "eu gu  ha hz h7  ea ek e7" --simulate-hands 20 --branching oracle --points --snapshotcache --position 3 --no-details

