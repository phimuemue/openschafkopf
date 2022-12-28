echo "Every other player has Herz:"
# Note that in the following command, we must be careful with player indices
./target/release/openschafkopf hand-stats --rules "Eichel-Solo von 2" --hand "eo so hu ez ek ha hk h9" --simulate-hands 1000000 --inspect "0<ctx.herz(1)&&0<ctx.herz(2)&&0<ctx.herz(3)"
echo "Player 3 has Herz after we already see Herz-Sieben and Herz-Acht"
# Remark: I am not sure, but I think there is a mistake in the book, as it weights (2,1,0) and (1,1,1) equally - which is incorrect imho.
# Thus, the following result remarkably diverges from the book.
./target/release/openschafkopf hand-stats --rules "Eichel-Solo von 2" --hand "eo so hu ez ek ha hk h9" --cards-on-table "h7 h8" --simulate-hands 1000000 --inspect "ctx.herz(3)"
