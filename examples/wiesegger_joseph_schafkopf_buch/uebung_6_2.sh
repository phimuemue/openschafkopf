./target/release/openschafkopf analyze ./examples/wiesegger_joseph_schafkopf_buch/uebung_6_2.txt
echo "As mentioned in the solution, player 1 might have thought that player 0 does not hold Eichel anymore:"
./target/release/openschafkopf suggest-card --rules "Wenz von 1" --cards-on-table "ga gu g7 s7  eu s9 h8 su  sa h7 s8 sz  ea ek eo e8" --hand "ha hz h9 ez" --branching "9,9" --simulate-hands all
echo "If player 1 knows that player 0 does not hold Eichel anymore:"
./target/release/openschafkopf suggest-card --rules "Wenz von 1" --cards-on-table "ga gu g7 s7  eu s9 h8 su  sa h7 s8 sz  ea ek eo e8" --hand "ha hz h9 ez" --branching "9,9" --simulate-hands all --constrain-hands '0==e(0)'
echo "If player 1 knows that player 0 does still hold Eichel, they can enforce a positive payout:"
./target/release/openschafkopf suggest-card --rules "Wenz von 1" --cards-on-table "ga gu g7 s7  eu s9 h8 su  sa h7 s8 sz  ea ek eo e8" --hand "ha hz h9 ez" --branching "9,9" --simulate-hands all --constrain-hands '0<e(0)'
