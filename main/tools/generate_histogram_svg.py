import subprocess
import sys
import json
import random

str_rules = "herz solo"

n_simulate_hands = 100

def generate_histogram(str_hand, str_epi):
    try:
        str_json = subprocess.run(
            [
                "target/release/openschafkopf",
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
    except FileNotFoundError:
        print("Error: openschafkopf not found.")
        sys.exit(1)

    json_data = json.loads(str_json)
    vecn_histogram = [0] * 121
    mapstrn_histogram = json_data["vectableline"][0]["amapnn_histogram"][2]
    for str_payout in json_data["vectableline"][0]["amapnn_histogram"][2]:
        vecn_histogram[int(str_payout)] += mapstrn_histogram[str_payout]

    n_lose_schneider = sum(vecn_histogram[:31])
    n_lose = sum(vecn_histogram[31:61])
    n_win = sum(vecn_histogram[61:90])
    n_win_schneider = sum(vecn_histogram[91:])

    print(str_epi + ": " + str((n_lose_schneider, n_lose, n_win, n_win_schneider)))

    def count_to_svg(n_count):
        return 100*n_count/n_simulate_hands

    def rect_y_height(n_count):
        return 'height="%d"\ny="%d"'%(count_to_svg(n_count), 101-count_to_svg(n_count))

    def path_detail(n_x_initial, vecn):
        return str(n_x_initial) + "," + str(int(101-count_to_svg(vecn[0]))) + " h 2" + " ".join(("v " + str(int(count_to_svg(n1)-count_to_svg(n2))) + " h 2 " for (n2, n1) in zip(vecn[1:], vecn[:-1])))

    str_svg = """<?xml version="1.0" encoding="UTF-8" standalone="no"?>
    <svg
       xmlns:dc="http://purl.org/dc/elements/1.1/"
       xmlns:cc="http://creativecommons.org/ns#"
       xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
       xmlns:svg="http://www.w3.org/2000/svg"
       xmlns="http://www.w3.org/2000/svg"
       width="244"
       height="102"
       viewBox="1 1 244.00001 102"
       version="1.1"
       id="svg8">
      <defs
         id="defs2" />
      <metadata
         id="metadata5">
        <rdf:RDF>
          <cc:Work
             rdf:about="">
            <dc:format>image/svg+xml</dc:format>
            <dc:type
               rdf:resource="http://purl.org/dc/dcmitype/StillImage" />
            <dc:title></dc:title>
          </cc:Work>
        </rdf:RDF>
      </metadata>
      <g
         id="layer1">
        <rect
           style="opacity:1;fill:#ff0000;fill-opacity:0.404747;stroke:none;stroke-width:1.14207;stroke-opacity:1;paint-order:fill markers stroke"
           id="rect1344"
           width="62"
           x="2"
    """ + rect_y_height(n_lose_schneider) + """
           />
        <rect
           style="opacity:1;fill:#ff0000;fill-opacity:0.404747;stroke:none;stroke-width:1.56888;stroke-opacity:1;paint-order:fill markers stroke"
           id="rect1346"
           width="60"
           x="64"
    """ + rect_y_height(n_lose) + """
           />
        <rect
           style="opacity:1;fill:#00ff00;fill-opacity:0.404747;stroke:none;stroke-width:1.1235;stroke-opacity:1;paint-order:fill markers stroke"
           id="rect1348"
           width="60"
           x="124"
    """ + rect_y_height(n_win) + """
           />
        <rect
           style="opacity:1;fill:#00ff00;fill-opacity:0.404747;stroke:none;stroke-width:1.56888;stroke-opacity:1;paint-order:fill markers stroke"
           id="rect1350"
           width="60"
           x="184"
    """ + rect_y_height(n_win_schneider) + """
           />
        <path
           id="rect835"
           style="opacity:1;fill:none;stroke:#ff0000;stroke-width:0.5;stroke-opacity:1;paint-order:fill markers stroke;stroke-miterlimit:4;stroke-dasharray:none"
           d="m""" + path_detail(1, vecn_histogram[:61]) + """ "
           />
        <path
           id="path1342"
           style="opacity:1;fill:none;stroke:#007500;stroke-width:0.5;stroke-opacity:1;paint-order:fill markers stroke;stroke-miterlimit:4;stroke-dasharray:none"
           d="m""" + path_detail(124, vecn_histogram[61:]) + """ "
           />
      </g>
    </svg>
    """

    str_filename = str_hand + " von " + str_epi
    with open(str_filename + "_histogram.svg", "w") as file_svg:
        file_svg.write(str_svg)
    with open(str_filename + "_histogram.txt", "w") as file_txt:
        file_txt.write(str(vecn_histogram))

vecstr_card = [str_efarbe + str_eschlag for str_efarbe in "eghs" for str_eschlag in "9zuoka"]
print(vecstr_card)

setsetstr_hand = {frozenset({"eo", "go", "ho", "so", "eu", "gu"})}

def mutate_hand(setstr_hand):
    setstr_hand_mut = set(setstr_hand)
    card_old = random.choice(list(setstr_hand_mut))
    card_new = random.choice([x for x in vecstr_card if x not in setstr_hand_mut])
    setstr_hand_mut.remove(card_old)
    setstr_hand_mut.add(card_new)
    return frozenset(setstr_hand_mut)

for setstr_hand in [mutate_hand(random.choice(list(setsetstr_hand))) for _ in range(6)]:
    setsetstr_hand.add(setstr_hand)
for setstr_hand in [mutate_hand(random.choice(list(setsetstr_hand))) for _ in range(12)]:
    setsetstr_hand.add(setstr_hand)
for setstr_hand in [mutate_hand(random.choice(list(setsetstr_hand))) for _ in range(24)]:
    setsetstr_hand.add(setstr_hand)
for setstr_hand in [mutate_hand(random.choice(list(setsetstr_hand))) for _ in range(24)]:
    setsetstr_hand.add(setstr_hand)

for setstr_hand in setsetstr_hand:
    str_hand = " ".join(setstr_hand)
    print(str_hand)

for setstr_hand in setsetstr_hand:
    str_hand = " ".join(setstr_hand)
    print(str_hand)
    generate_histogram(str_hand, "0")
    generate_histogram(str_hand, "1")
    generate_histogram(str_hand, "2")
    generate_histogram(str_hand, "3")
