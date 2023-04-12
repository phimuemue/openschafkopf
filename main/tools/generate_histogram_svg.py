#!/usr/bin/python3

import subprocess
import sys
import json
import random
import os

str_path_openschafkopf = "target/release/openschafkopf"
str_rules = "wenz"
n_simulate_hands = 100

def generate_histogram(str_hand, str_epi):
    str_json = subprocess.run(
        [
            str_path_openschafkopf,
            "suggest-card",
            "--hand", str_hand,
            "--rules", str_rules + " von " + str_epi,
            "--position", str_epi,
            "--simulate-hands", str(n_simulate_hands),
            "--points",
            "--snapshotcache",
            "--branching", "equiv4",
            "--no-details",
            "--json",
        ],
        # capture_output=True, only avaiable in python 3.7
        stdout=subprocess.PIPE,
    ).stdout.decode()
    str_path_dir = "histograms/" + str_rules + "/" + str_hand
    os.makedirs(str_path_dir, exist_ok=True)
    with open(str_path_dir + "/" + str_epi + ".json", "w") as file_json:
        file_json.write(str_json)

vecstr_card = [str_efarbe + str_eschlag for str_efarbe in "eghs" for str_eschlag in "9zuoka"]

setsetstr_hand = {frozenset({"eu", "gu", "hu", "su", "ea", "ga"})}

def mutate_hand(setstr_hand, n):
    setstr_hand_mut = set(setstr_hand)
    for _ in range(n):
        card_old = random.choice(list(setstr_hand_mut))
        card_new = random.choice([x for x in vecstr_card if x not in setstr_hand_mut])
        setstr_hand_mut.remove(card_old)
        setstr_hand_mut.add(card_new)
    return frozenset(setstr_hand_mut)

for setstr_hand in [mutate_hand(random.choice(list(setsetstr_hand)), 1) for i in range(24)]:
    setsetstr_hand.add(setstr_hand)
for setstr_hand in [mutate_hand(random.choice(list(setsetstr_hand)), 2) for i in range(24)]:
    setsetstr_hand.add(setstr_hand)
for setstr_hand in [mutate_hand(random.choice(list(setsetstr_hand)), 3) for i in range(24)]:
    setsetstr_hand.add(setstr_hand)

def canonical_hand_string(setstr_hand):
    return subprocess.run(
        [
            str_path_openschafkopf,
            "hand-stats",
            "--hand", " ".join(setstr_hand),
            "--rules", str_rules + " von 0",
            "--inspect", "ctx.hand_to_string(0)",
            "--simulate-hands", "1",
        ],
        stdout=subprocess.PIPE,
    ).stdout.decode()[:17]

for setstr_hand in setsetstr_hand:
    str_hand = canonical_hand_string(setstr_hand)
    print(str_hand)
    generate_histogram(str_hand, "0")
    generate_histogram(str_hand, "1")
    generate_histogram(str_hand, "2")
    generate_histogram(str_hand, "3")
