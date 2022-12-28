# Takes too long to wait for it.

./target/release/openschafkopf suggest-card --rules "Eichel-Rufspiel von 2" --cards-on-table "ea e7 ez e9  sa sk s7 s8  g7 gz" --hand "go ho eu su hz h7" --simulate-hands all --branching equiv7 --verbose
./target/release/openschafkopf suggest-card --rules "Eichel-Rufspiel von 2" --cards-on-table "ea e7 ez e9  sa sk s7 s8  g7 gz" --hand "go ho eu su hz h7" --simulate-hands all --branching equiv7 --verbose --constrain-hands '0==ctx.gras(3)'
./target/release/openschafkopf suggest-card --rules "Eichel-Rufspiel von 2" --cards-on-table "ea e7 ez e9  sa sk s7 s8  g7 gz" --hand "go ho eu su hz h7" --simulate-hands all --branching equiv7 --verbose --constrain-hands '(0==ctx.gras(3))&&ctx.trumpf(3)>4'
