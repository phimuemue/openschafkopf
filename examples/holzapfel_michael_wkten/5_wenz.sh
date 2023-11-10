set -e

echo "https://www.michael-holzapfel.de/schk/ws5-U/ws-Wenz.htm - Relative Haeufigkeiten beim Wenz"

N_SIMULATE_HANDS=1000

echo "1. Fall: 4 Unter, Koenig und Ober in einer Farbe"
./target/release/openschafkopf suggest-card --simulate-hands $N_SIMULATE_HANDS  --rules "wenz von 0" --hand "eu gu hu su ek eo" --points --snapshotcache --verbose --branching equiv5

echo ""
echo "2. Fall: 4 Unter, Koenig und Ober in 2 verschiedenen Farben"
./target/release/openschafkopf suggest-card --simulate-hands $N_SIMULATE_HANDS  --rules "wenz von 0" --hand "eu gu hu su ek go" --points --snapshotcache --branching equiv5

echo ""
echo "3. Fall: 3 Unter, Herz-Ass, Koenig und Ober in 2 verschiedenen Nicht-Herz-Farben"
./target/release/openschafkopf suggest-card --simulate-hands $N_SIMULATE_HANDS  --rules "wenz von 0" --hand "eu gu hu ha ek go" --points --snapshotcache --branching equiv5
