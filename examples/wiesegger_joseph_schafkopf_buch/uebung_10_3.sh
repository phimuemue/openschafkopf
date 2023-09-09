# Remark: The solution's argument for Gras-Koenig is somewhat convincing - but the software cannot capture it as of now.

./target/release/openschafkopf suggest-card --rules "Eichel-Wenz von 2" --cards-on-table "ha hk ea h8  gu ez eu e7  sk s7 ek sa  su g7 e8 hu" --hand "ga gk go g8" --simulate-hands all --branching "9,9" --verbose --points --snapshotcache

echo ""
echo "The above shows that we cannot win this game by our own efforts."
echo ""
echo "Links' cards can be determined:"
./target/release/openschafkopf hand-stats --rules "Eichel-Wenz von 2" --cards-on-table "ha hk ea h8  gu ez eu e7  sk s7 ek sa  su g7 e8 hu" --hand "ga gk go g8" --simulate-hands all --inspect "ctx.hand_to_string(2)"

echo ""
echo "Run simulation from Links' point of view, depending on the card we play:"
for card in GA GK GO G8
do
    echo "If we play $card:"
    ./target/release/openschafkopf suggest-card \
        --rules "Eichel-Wenz von 2" \
        --cards-on-table "ha hk ea h8  gu ez eu e7  sk s7 ek sa  su g7 e8 hu $card" \
        --hand "eo e9 gz g9" \
        --no-details \
        --points --snapshotcache
done
echo "The above shows that we should not play Gras-Ass but some other card."

echo "Can also be confirmed more comfortably:"
./target/release/openschafkopf suggest-card \
    --rules "Eichel-Wenz von 2" \
    --cards-on-table "ha hk ea h8  gu ez eu e7  sk s7 ek sa  su g7 e8 hu" \
    --hand "eo e9 gz g9" \
    --points \
    --position 2
        
