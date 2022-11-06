./target/release/openschafkopf hand-stats --rules "Eichel Rufspiel von 3" --cards-on-table "g8 gz g9 ha  go eo ga h7  s9 sa s7 sz" --hand "sk s8 e7 gk g7" --simulate-hands all --inspect "t(3)"
echo "Assuming that player 3 has Eichel-Zehn blank confirms solution"
./target/release/openschafkopf hand-stats --rules "Eichel Rufspiel von 3" --cards-on-table "g8 gz g9 ha  go eo ga h7  s9 sa s7 sz" --hand "sk s8 e7 gk g7" --simulate-hands all --inspect "t(3)" --constrain-hands "ez(3)&e(3)==1"
