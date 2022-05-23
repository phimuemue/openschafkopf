./target/release/openschafkopf suggest-card --rules "Herz-Wenz von 0" --cards-on-table "gu ho h7 h9  h8 eu sz sk  ga g7 go ha  hu e7 g8 s9  eo gz e9" --hand "ea ek e8 so" --simulate-hands all --branching equiv7 --verbose
echo "We confirm that Eichel-Zehn is at player 2"
./target/release/openschafkopf hand-stats --rules "Herz-Wenz von 0" --cards-on-table "gu ho h7 h9  h8 eu sz sk  ga g7 go ha  hu e7 g8 s9  eo gz e9" --hand "ea ek e8 so" --simulate-hands all --inspect "ez(0)" --inspect "ez(1)" --inspect "ez(2)" --inspect "ez(3)"
