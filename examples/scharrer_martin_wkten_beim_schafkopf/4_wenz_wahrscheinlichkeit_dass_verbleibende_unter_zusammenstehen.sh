set -e

N_SIMULATE_HANDS=10000

echo "4 Wenz: Wahrscheinlichkeit, dass verbleibende Unter zusammenstehen - bestätigt"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "wenz von 0" \
    --hand "eu gu   ea ez ek eo e9 e8" \
    --hand "eu      ea ez ek eo e9 e8 e7" \
    --inspect "/*Gegner hat 2 Unter*/ ctx.trumpf().extract(1).contains(2)" \
    --inspect "/*Gegner hat 3 Unter*/ ctx.trumpf().extract(1).contains(3)"
