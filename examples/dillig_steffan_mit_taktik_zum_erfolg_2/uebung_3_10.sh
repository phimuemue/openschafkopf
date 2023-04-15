#!/bin/bash

./target/release/openschafkopf suggest-card --rules "0 spielt mit der Eichel-Ass" --hand "so eu hu su hk  e8  ga gz" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache --no-details

echo "What about substituting E8 with a H7?"

./target/release/openschafkopf suggest-card --rules "0 spielt mit der Eichel-Ass" --hand "so eu hu su hk h7  ga gz" --simulate-hands 20 --branching oracle --points --snapshotcache --no-details

echo "What about 6 Trumpf, ohne 3, two Eichel?"

./target/release/openschafkopf suggest-card --rules "0 spielt mit der Eichel-Ass" --hand "so eu hu su hk h7  ez e7" --simulate-hands 20 --branching oracle --points --snapshotcache --no-details
