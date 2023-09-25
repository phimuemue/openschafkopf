echo 'Rufspiel mit der Schelln-Sau von 0'
echo 'Stichs so far:'
echo '>EO H9 HA HO'
echo '>GO HU H7 S7'
echo '>SO HZ GU E7'
echo '>H8 EU G7 GA'
echo ' __>G8 S8 GZ'
echo 'Hand: SU HK E9 SZ'
echo 'NetSchafkopf suggests HK'
echo "Raw payout"
./target/release/openschafkopf suggest-card --rules "Rufspiel mit der Schelln-Sau von 0" --cards-on-table "EO H9 HA HO  GO HU H7 S7  SO HZ GU E7  H8 EU G7 GA  G8 S8 GZ" --hand "SU HK E9 SZ" --branching "equiv7"
echo "Points"
./target/release/openschafkopf suggest-card --rules "Rufspiel mit der Schelln-Sau von 0" --cards-on-table "EO H9 HA HO  GO HU H7 S7  SO HZ GU E7  H8 EU G7 GA  G8 S8 GZ" --hand "SU HK E9 SZ" --branching "equiv7" --points
echo "Raw payout, assuming that 2 has Schelln-Sau"
./target/release/openschafkopf suggest-card --rules "Rufspiel mit der Schelln-Sau von 0" --cards-on-table "EO H9 HA HO  GO HU H7 S7  SO HZ GU E7  H8 EU G7 GA  G8 S8 GZ" --hand "SU HK E9 SZ" --branching "equiv7" --constrain-hands "ctx.who_has_sa()==2"
echo "Points, assuming that 2 has Schelln-Sau"
./target/release/openschafkopf suggest-card --rules "Rufspiel mit der Schelln-Sau von 0" --cards-on-table "EO H9 HA HO  GO HU H7 S7  SO HZ GU E7  H8 EU G7 GA  G8 S8 GZ" --hand "SU HK E9 SZ" --branching "equiv7" --constrain-hands "ctx.who_has_sa()==2" --points
