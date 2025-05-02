
Hand | Gewinnwahrscheinlichkeit
-- | -
eu gu hu su ha hz | 100.0%
eu gu hu su ha ho | 100.0%
eu gu hu su ha hk | 100.0%
eu gu hu su ha h9 | 100.0%
eu gu hu su hz ho | 100.0%
eu gu hu su hz hk | 100.0%
eu gu hu su hz h9 | 100.0%
eu gu hu su ho hk | 100.0%
eu gu hu su ho h9 | 100.0%
eu gu hu su hk h9 | 100.0%
eu gu hu ha hz ho | 100.0%
eu gu hu ha hz hk | 100.0%
eu gu hu ha hz h9 | 100.0%
eu gu hu ha ho hk | 100.0%
eu gu hu ha ho h9 | 100.0%
eu gu hu ha hk h9 | 100.0%
eu gu hu hz ho hk | 100.0%
eu gu hu hz ho h9 | 100.0%
eu gu hu hz hk h9 | 100.0%
eu gu hu ho hk h9 | 100.0%
eu gu su ha hz ho | 92.21000000000001%
eu gu su ha hz hk | 92.9%
eu gu su ha hz h9 | 92.54%
eu gu su ha ho hk | 92.97999999999999%
eu gu su ha ho h9 | 92.97999999999999%
eu gu su ha hk h9 | 92.30000000000001%
eu gu su hz ho hk | 92.39%
eu gu su hz ho h9 | 92.35%
eu gu su hz hk h9 | 92.60000000000001%
eu gu su ho hk h9 | 92.60000000000001%
eu gu ha hz ho hk | 92.47%
eu gu ha hz ho h9 | 92.56%
eu gu ha hz hk h9 | 93.15%
eu gu ha ho hk h9 | 92.65%
eu gu hz ho hk h9 | 92.51%
eu hu su ha hz ho | 48.620000000000005%
eu hu su ha hz hk | 49.58%
eu hu su ha hz h9 | 48.44%
eu hu su ha ho hk | 48.19%
eu hu su ha ho h9 | 48.38%
eu hu su ha hk h9 | 49.45%
eu hu su hz ho hk | 48.88%
eu hu su hz ho h9 | 48.9%
eu hu su hz hk h9 | 48.03%
eu hu su ho hk h9 | 48.93%
eu hu ha hz ho hk | 48.9%
eu hu ha hz ho h9 | 49.02%
eu hu ha hz hk h9 | 49.51%
eu hu ha ho hk h9 | 48.44%
eu hu hz ho hk h9 | 48.54%
eu su ha hz ho hk | 26.35%
eu su ha hz ho h9 | 26.240000000000002%
eu su ha hz hk h9 | 26.33%
eu su ha ho hk h9 | 25.91%
eu su hz ho hk h9 | 26.82%
eu ha hz ho hk h9 | 27.029999999999998%

```
import itertools
import subprocess

veccard_trumpf = "eu gu hu su ha hz ho hk h9".split(" ")

# open table
print("") # according to https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/organizing-information-with-tables, a newline is necessary before tables
print("Hand | Gewinnwahrscheinlichkeit")
print("-- | -")

for veccard_hand in itertools.combinations(veccard_trumpf, 6):
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

```
