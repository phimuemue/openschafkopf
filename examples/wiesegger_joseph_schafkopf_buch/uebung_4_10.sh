# first run is --verbose so you get a feeling how long the computation will take

./target/release/openschafkopf suggest-card --rules "eichel solo von 0" --cards-on-table "e8 ea hu su  sa hk ez s7  e9 eu e7 gu  sk sz eo" --hand "go so ek ga gz" --simulate-hands all --branching equiv7 --verbose

echo "Above analysis shows that SO and EK lead to same payouts, but we can observe that point-wise, SO is the better choice:"
./target/release/openschafkopf suggest-card --rules "eichel solo von 0" --cards-on-table "e8 ea hu su  sa hk ez s7  e9 eu e7 gu  sk sz eo" --hand "go so ek ga gz" --simulate-hands all --branching equiv7 --points

echo "As mentioned in uebung 4.11, we suspect that player 3 does not hold Herz-Ober"
./target/release/openschafkopf suggest-card --rules "eichel solo von 0" --cards-on-table "e8 ea hu su  sa hk ez s7  e9 eu e7 gu  sk sz eo" --hand "go so ek ga gz" --simulate-hands all --branching equiv7 --constrain-hands '0==ctx.ho(3)'

echo "Above analysis shows that SO and EK lead to same payouts, but we can observe that point-wise, SO is the better choice:"
./target/release/openschafkopf suggest-card --rules "eichel solo von 0" --cards-on-table "e8 ea hu su  sa hk ez s7  e9 eu e7 gu  sk sz eo" --hand "go so ek ga gz" --simulate-hands all --branching equiv7 --constrain-hands '0==ctx.ho(3)' --points
