import itertools
import subprocess

veccard_trumpf = "eu gu hu su ha hz ho hk h9 h8 h7".split(" ")

# open table
print("") # according to https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/organizing-information-with-tables, a newline is necessary before tables
print("Hand | Gewinnwahrscheinlichkeit")
print("-- | -")

for veccard_hand in itertools.combinations(veccard_trumpf, 8):
    if veccard_trumpf[0] not in veccard_hand: # inefficient but makes code simpler
        continue
    str_hand = " ".join(veccard_hand)
    n_high_trumpf_with_active_player = 0
    vecstr_opponent_is_weak_enough = []
    for card_trumpf in veccard_trumpf:
        if card_trumpf in veccard_hand:
            n_high_trumpf_with_active_player = n_high_trumpf_with_active_player + 1
        else:
            vecstr_opponent_is_weak_enough.append(f"ctx.trumpf(ctx.who_has_{card_trumpf}()) <= {n_high_trumpf_with_active_player}")
    str_openschafkopf_output = subprocess.run(
        [
            "target/release/openschafkopf",
            "hand-stats",
            "--rules", "herz wenz von 0",
            "--hand", str_hand,
            "--simulate-hands", "10000",
            "--inspect", " && ".join(vecstr_opponent_is_weak_enough),
            # "--json", # TODO support json in hand-stats
        ],
        capture_output=True,
        text=True,
        check=True
    ).stdout
    print(str_hand + " | " + str(float(str_openschafkopf_output.splitlines()[-1].split(" ")[1])*100) + "%")

print("") # the table

# for reference, also print this file's contents
print("```")
with open(__file__, "r") as file_this_script:
    print(file_this_script.read())
print("```")
