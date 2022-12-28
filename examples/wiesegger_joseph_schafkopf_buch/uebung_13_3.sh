./target/release/openschafkopf suggest-card --rules "Gras-Wenz von 3" --cards-on-table "ha h9 h7 gz  gu eu g7 ga  s7 s9 sa go  su gk hk g8  e7 ek ez sz e9" --hand "hz h8 so" --simulate-hands all --branching "9,9" --verbose
echo "Now excluding the possibility of player 1 has Eichel-Ass"
./target/release/openschafkopf suggest-card --rules "Gras-Wenz von 3" --cards-on-table "ha h9 h7 gz  gu eu g7 ga  s7 s9 sa go  su gk hk g8  e7 ek ez sz e9" --hand "hz h8 so" --simulate-hands all --branching "9,9" --constrain-hands '0==ctx.ea(1)'
