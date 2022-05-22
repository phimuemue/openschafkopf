./target/release/openschafkopf suggest-card --rules "Gras Rufspiel von 0" --cards-on-table "h7 ho h9 go  eu h8 so su  g7 gk ga gz  sa g8 sk hu  ek ea" --hand "eo ha hz g9" --simulate-hands all --branching "9,9" --verbose
echo "Same computation point-wise - surprisingly, G9 lands between HA and HZ:"
./target/release/openschafkopf suggest-card --rules "Gras Rufspiel von 0" --cards-on-table "h7 ho h9 go  eu h8 so su  g7 gk ga gz  sa g8 sk hu  ek ea" --hand "eo ha hz g9" --simulate-hands all --branching "9,9" --points
