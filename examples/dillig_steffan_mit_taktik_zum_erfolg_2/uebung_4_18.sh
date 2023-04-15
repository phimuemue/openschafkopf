#!/bin/bash

./target/release/openschafkopf suggest-card --rules "0 spielt Herz-Wenz" --hand "eu gu hu su ha h9 h7  g7" --simulate-hands 20 --branching oracle --points --snapshotcache --verbose

