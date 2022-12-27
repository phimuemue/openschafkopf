set -e

N_SIMULATE_HANDS=10000

echo "6 Stechen mit Farben - bestätigt"

echo "6.1 Sau und 10er"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS --rules "herz-solo von 0" \
    --hand "eo go ho so eu   ea ez e9" \
    --inspect "/*Ein Spieler ausser 0 hat 3 Eichel*/ ctx.eichel().extract(1).contains(3)"

echo "6.2 Sau und König"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS --rules "herz-solo von 0" \
    --hand "eo go ho so eu gu   ea ek" \
    --inspect "/*Der Spieler mit der Eichel-Zehn hat nur 1 Eichel*/ ctx.eichel(ctx.ez().index_of(1))==1"

echo "6.3 Sau und König plus Eins"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS --rules "herz-solo von 0" \
    --hand "eo go ho so eu      ea ek e9" \
    --inspect "/*Der Spieler mit der Eichel-Zehn hat nur 1 Eichel*/ ctx.eichel(ctx.ez().index_of(1))==1"

echo "6.4 Sau und König plus Zwei"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS --rules "herz-solo von 0" \
    --hand "eo go ho so         ea ek e9 e8" \
    --inspect "/*Der Spieler mit der Eichel-Zehn hat nur 1 Eichel*/ ctx.eichel(ctx.ez().index_of(1))==1"

echo "6.5 Sau und 9er"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS --rules "herz-solo von 0" \
    --hand "eo go ho so eu gu   ea e9" \
    --inspect "/*Der Spieler mit der Eichel-Zehn und mit Eichel-Koenig haben je nur 1 Eichel*/ ctx.eichel(ctx.ez().index_of(1))==1 && ctx.eichel(ctx.ek().index_of(1))==1"

echo "6.6 Sau und 9er plus Eins"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS --rules "herz-solo von 0" \
    --hand "eo go ho so eu      ea e9 e8" \
    --inspect "/*Der Spieler mit der Eichel-Zehn und mit Eichel-Koenig haben je nur 1 Eichel*/ ctx.eichel(ctx.ez().index_of(1))==1 && ctx.eichel(ctx.ek().index_of(1))==1"

echo "6.6 Sau und 9er, 8er, 7er"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS --rules "herz-solo von 0" \
    --hand "eo go ho so         ea e9 e8 e7" \
    --inspect "ctx.ek()!=ctx.ez()"
