#!/bin/bash

# to be run from the openschafkopf directory

for STR_FILE in `ls ../openschafkopf_training_data/raw`
do
    7z x ../openschafkopf_training_data/raw/$STR_FILE -otmp
    ./target/release/openschafkopf parse tmp/**/*.html > ../openschafkopf_training_data/rules_stichseq/$STR_FILE.txt
    rm -rf tmp
done

