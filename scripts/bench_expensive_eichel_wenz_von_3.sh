#!/bin/bash

./target/release/openschafkopf suggest-card --hand "GA GZ G8 HA HK H8 H7 SO  GU GK GO G9 G7 H9 SK S8  EU HU EA E9 HZ HO SZ S9  SU EZ EK EO E8 E7 SA S7" --rules "eichel wenz von 3" --points --snapshotcache --verbose --branching oracle | ts
