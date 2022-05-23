./target/release/openschafkopf suggest-card --rules "Eichel-Wenz von 1" --cards-on-table "ga e8 eo gk  sk so ez sa  gz su e7 g7  eu s7 g9 ek  gu s8 h8 hu  e9 hz" --hand "ha ho h9" --simulate-hands all --branching equiv7 --verbose
echo "Assuming that Eichel-Ass is with player 1:"
./target/release/openschafkopf suggest-card --rules "Eichel-Wenz von 1" --cards-on-table "ga e8 eo gk  sk so ez sa  gz su e7 g7  eu s7 g9 ek  gu s8 h8 hu  e9 hz" --hand "ha ho h9" --simulate-hands all --branching equiv7 --constrain-hands "ea(1)"
