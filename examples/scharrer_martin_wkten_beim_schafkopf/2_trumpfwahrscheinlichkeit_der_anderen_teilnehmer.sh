set -e

N_SIMULATE_HANDS=10000

echo "2.3 Trumpfwahrscheinlichkeit der anderen Teilnehmer - bestätigt"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "herz-solo von 0" \
    --hand "eo go ho so eu gu hu su" \
    --hand "eo go ho so eu gu hu    e7" \
    --hand "eo go ho so eu gu       e7 e8" \
    --hand "eo go ho so eu          e7 e8 e9" \
    --hand "eo go ho so             e7 e8 e9 ek" \
    --hand "eo go ho                e7 e8 e9 ek ez" \
    --hand "eo go                   e7 e8 e9 ek ez ea" \
    --hand "eo                      e7 e8 e9 ek ez ea ga" \
    --hand "                        e7 e8 e9 ek ez ea ga gz" \
    --inspect "ctx.trumpf(1)"
