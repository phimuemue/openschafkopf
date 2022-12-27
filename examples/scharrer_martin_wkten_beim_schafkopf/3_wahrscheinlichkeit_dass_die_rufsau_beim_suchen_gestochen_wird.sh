set -e

N_SIMULATE_HANDS=10000

echo "3 Wahrscheinlichkeit, dass die Rufsau beim Suchen gestochen wird - teilweise abweichend"

echo "3.1 Gleichverteilte Karten - abweichend"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "eichel rufspiel von 1" \
    --hand "eo go ho so eu gu hu    e7" \
    --hand "eo go ho so eu gu       e7 e8" \
    --hand "eo go ho so eu          e7 e8 e9" \
    --hand "eo go ho so             e7 e8 e9 ek" \
    --inspect "/*Irgendjemand Eichel frei?*/ ctx.eichel().contains(0)" \
    --inspect "/*Ist Spieler 2 mein Partner und Eichel frei?*/ ctx.eichel(2) ==0" \
    --inspect "/*Irgendjemand Eichel frei und mit Trumpf?*/ ctx.eichel().contains(0) && ctx.trumpf(ctx.eichel().index_of(0))>0" \
    --inspect "/*Ist Spieler 2 Eichel frei und hat Trumpf?*/ ctx.eichel(2)==0 && ctx.trumpf(2)>0"

echo "3.2 Spieler hat nur wenige Karten in der Ruffarbe - vermutlich bestätigt"
echo "Fall s == 1"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "eichel rufspiel von 1" \
    --hand "eo go ho so eu gu hu    e7" \
    --hand "eo go ho so eu gu       e7 e8" \
    --hand "eo go ho so eu          e7 e8 e9" \
    --hand "eo go ho so             e7 e8 e9 ek" \
    --constrain-hands "ctx.eichel(1)==1" \
    --inspect "/*Irgendjemand Eichel frei?*/ ctx.eichel().contains(0)" \
    --inspect "/*Irgendjemand Eichel frei und mit Trumpf?*/ ctx.eichel().contains(0) && ctx.trumpf(ctx.eichel().index_of(0))>0"
echo "Fall s == 2"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "eichel rufspiel von 1" \
    --hand "eo go ho so eu gu hu    e7" \
    --hand "eo go ho so eu gu       e7 e8" \
    --hand "eo go ho so eu          e7 e8 e9" \
    --constrain-hands "ctx.eichel(1)==2" \
    --inspect "/*Irgendjemand Eichel frei?*/ ctx.eichel().contains(0)" \
    --inspect "/*Irgendjemand Eichel frei und mit Trumpf?*/ ctx.eichel().contains(0) && ctx.trumpf(ctx.eichel().index_of(0))>0"
echo "Fall s == 3"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "eichel rufspiel von 1" \
    --hand "eo go ho so eu gu hu    e7" \
    --hand "eo go ho so eu gu       e7 e8" \
    --constrain-hands "ctx.eichel(1)==3" \
    --inspect "/*Irgendjemand Eichel frei?*/ ctx.eichel().contains(0)" \
    --inspect "/*Irgendjemand Eichel frei und mit Trumpf?*/ ctx.eichel().contains(0) && ctx.trumpf(ctx.eichel().index_of(0))>0"
