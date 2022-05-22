echo "As mentioned in the solution, we can assume that player 3 holds Eichel-Ass, so we use that knowledge:"
./target/release/openschafkopf suggest-card --rules "Eichel Rufspiel von 0" --cards-on-table "eo su h7 so  go ho h8 ga  eu g7 h9 gz  s9 s8" --hand "_ _ _ _  _ _ _ _  gu hu hk gk g9  ea _ _ _ _" --simulate-hands all --branching equiv7 --verbose
echo "However, even without the knowledge about Eichel-Ass, we should play Herz-Koenig (warning: takes long):"
./target/release/openschafkopf suggest-card --rules "Eichel Rufspiel von 0" --cards-on-table "eo su h7 so  go ho h8 ga  eu g7 h9 gz  s9 s8" --hand "gu hu hk gk g9" --simulate-hands all --branching equiv7
