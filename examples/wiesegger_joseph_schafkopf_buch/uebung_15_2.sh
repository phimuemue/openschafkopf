echo "Approximating:"
./target/release/openschafkopf hand-stats --rules "Herz-Solo von 0" --hand "eo go ho so ea ez ga sa" --simulate-hands 1000000 --inspect "ctx.trumpf(1)<5&&ctx.trumpf(2)<5&&ctx.trumpf(3)<5"
echo "Enumerating all (takes too long):"
./target/release/openschafkopf hand-stats --rules "Herz-Solo von 0" --hand "eo go ho so ea ez ga sa" --simulate-hands all --inspect "ctx.trumpf(1)<5&&ctx.trumpf(2)<5&&ctx.trumpf(3)<5"
