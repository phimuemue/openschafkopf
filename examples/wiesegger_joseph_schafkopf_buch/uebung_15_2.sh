echo "Approximating:"
./target/release/openschafkopf hand-stats --rules "Herz-Solo von 0" --hand "eo go ho so ea ez ga sa" --simulate-hands 1000000 --inspect "t(1)<5&&t(2)<5&&t(3)<5"
echo "Enumerating all (takes too long):"
./target/release/openschafkopf hand-stats --rules "Herz-Solo von 0" --hand "eo go ho so ea ez ga sa" --simulate-hands all --inspect "t(1)<5&&t(2)<5&&t(3)<5"
