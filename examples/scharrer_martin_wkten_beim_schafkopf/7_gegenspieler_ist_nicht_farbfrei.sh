set -e

N_SIMULATE_HANDS=10000

echo "7 Gegenspieler ist nicht Farbfrei - abweichend"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS --rules "schelln rufspiel von 0" \
    --hand "eo go ho so eu hu   s7   ea" \
    --hand "eo go ho so eu      s7   ea ez" \
    --hand "eo go ho so         s7   ea ez ek" \
    --hand "eo go ho            s7   ea ez ek e9" \
    --hand "eo go               s7   ea ez ek e9 e8" \
    --inspect "let an_eichel = ctx.eichel(); an_eichel.remove(ctx.sa().index_of(1)); an_eichel.extract(1).contains(0)" \
    --inspect "[ctx.sa().index_of(1), ctx.eichel(), {let an_eichel = ctx.eichel(); an_eichel.remove(ctx.sa().index_of(1)); an_eichel.extract(1).contains(0)}]"

