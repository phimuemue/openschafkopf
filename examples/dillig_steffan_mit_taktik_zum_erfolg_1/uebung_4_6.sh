#!/bin/bash

./target/release/openschafkopf suggest-card --rules "3 spielt Herz-Solo" --hand "eo ho so hu su ha hk  gk" --cards-on-table "ea e9 ek" --simulate-hands 20 --branching oracle --verbose --points --snapshotcache

