./target/release/openschafkopf suggest-card --rules "Eichel-Wenz von 3" --cards-on-table "s7 sa so ea  su hu gu ek  sz e7 eo g9  e9 eu sk ga  gz g7 g8 ez  e8" --hand "go ha h9" --branching "9,9" --simulate-hands all
echo "The same computation point-wise:"
./target/release/openschafkopf suggest-card --rules "Eichel-Wenz von 3" --cards-on-table "s7 sa so ea  su hu gu ek  sz e7 eo g9  e9 eu sk ga  gz g7 g8 ez  e8" --hand "go ha h9" --branching "9,9" --simulate-hands all --points
