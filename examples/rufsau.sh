set -e

N_SIMULATE_HANDS=10000

echo "Einfache Verteilung der Eichel-Karten. Jemand frei?"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "herz-solo von 0" \
    --hand "eo go ho so eu gu hu su" \
    --hand "eo go ho so eu gu hu    e7" \
    --hand "eo go ho so eu gu       e7 e8" \
    --hand "eo go ho so eu          e7 e8 e9" \
    --hand "eo go ho so             e7 e8 e9 ek" \
    --hand "eo go ho                e7 e8 e9 ek ez" \
    --hand "eo go                   e7 e8 e9 ek ez ea" \
    --inspect "let vecn_eichel = ctx.eichel().extract(1); vecn_eichel.sort(); vecn_eichel.reverse(); vecn_eichel" \
    --inspect "let vecn_eichel = ctx.eichel().extract(1); vecn_eichel.contains(0)"

echo
echo "Eichel-Rufspiel von 1 (gegen 0) - ist Partner von 0 Eichel frei?"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "eichel rufspiel von 1" \
    --hand "eo go ho so eu gu hu su" \
    --hand "eo go ho so eu gu hu     e7" \
    --hand "eo go ho so eu gu        e7 e8" \
    --hand "eo go ho so eu           e7 e8 e9" \
    --hand "eo go ho so              e7 e8 e9 ek" \
    --inspect "let vecn_eichel = ctx.eichel().extract(1); vecn_eichel.contains(0)"

echo
echo "Eichel-Rufspiel von 1 (mit 0) - ist Gegner Eichel frei?"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "eichel rufspiel von 1" \
    --hand "eo go ho so eu gu hu   ea" \
    --hand "eo go ho so eu gu      ea e7" \
    --hand "eo go ho so eu         ea e7 e8" \
    --hand "eo go ho so            ea e7 e8 e9" \
    --hand "eo go ho               ea e7 e8 e9 ek" \
    --inspect "let vecn_eichel = ctx.eichel().extract(1); vecn_eichel.contains(0)"

echo
echo "Eichel-Rufspiel von 1 (gegen 0) (mind. 4 Trumpf, Eichel nicht längste Farbe) - ist Partner von 0 Eichel frei?"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "eichel rufspiel von 1" --constrain-hands "ctx.trumpf(1)>=4 && (ctx.gras(1)==0 || ctx.eichel(1)<=ctx.gras(1)) && (ctx.schelln(1)==0 || ctx.eichel(1)<=ctx.schelln(1))" \
    --hand "eo go ho so eu gu hu su" \
    --hand "eo go ho so eu gu hu     e7" \
    --hand "eo go ho so eu gu        e7 e8" \
    --hand "eo go ho so eu           e7 e8 e9" \
    --hand "eo go ho so              e7 e8 e9 ek" \
    --inspect "let vecn_eichel = ctx.eichel().extract(1); vecn_eichel.contains(0)"

echo
echo "Eichel-Rufspiel von 1 (mit 0) (mind. 4 Trumpf, Eichel nicht längste Farbe) - ist Gegner von 0 Eichel frei?"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "eichel rufspiel von 1" --constrain-hands "ctx.trumpf(1)>=4 && (ctx.gras(1)==0 || ctx.eichel(1)<=ctx.gras(1)) && (ctx.schelln(1)==0 || ctx.eichel(1)<=ctx.schelln(1))" \
    --hand "go ho so eu gu hu su  ea" \
    --hand "go ho so eu gu hu     ea e7" \
    --hand "go ho so eu gu        ea e7 e8" \
    --hand "go ho so eu           ea e7 e8 e9" \
    --hand "go ho so              ea e7 e8 e9 ek" \
    --inspect "let vecn_eichel = ctx.eichel().extract(1); vecn_eichel.contains(0)"

echo
echo "Eichel-Rufspiel von 1 (gegen 0) (mind. 4 Trumpf, Eichel nicht längste Farbe) - ist Partner von 0 Eichel frei und hat Trumpf?"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "eichel rufspiel von 1" --constrain-hands "ctx.trumpf(1)>=4 && (ctx.gras(1)==0 || ctx.eichel(1)<=ctx.gras(1)) && (ctx.schelln(1)==0 || ctx.eichel(1)<=ctx.schelln(1))" \
    --hand "eo go ho so eu gu hu su" \
    --hand "eo go ho so eu gu hu     ga" \
    --hand "eo go ho so eu gu        ga gz" \
    --hand "eo go ho so eu           ga gz gk" \
    --hand "eo go ho so              ga gz gk g9" \
    --hand "eo go ho                 ga gz gk g9 g8" \
    --hand "eo go                    ga gz gk g9 g8 g7" \
    --hand "eo                       ga gz gk g9 g8 g7  sa" \
    --hand "                         ga gz gk g9 g8 g7  sa sz" \
    \
    --hand "go ho so eu gu hu su  e7" \
    --hand "go ho so eu gu hu     e7  ga" \
    --hand "go ho so eu gu        e7  ga gz" \
    --hand "go ho so eu           e7  ga gz gk" \
    --hand "go ho so              e7  ga gz gk g9" \
    --hand "go ho                 e7  ga gz gk g9 g8" \
    --hand "go                    e7  ga gz gk g9 g8 g7" \
    --hand "                      e7  ga gz gk g9 g8 g7 sa" \
    \
    --hand "go ho so eu gu hu  e7 e8" \
    --hand "go ho so eu gu     e7 e8  ga" \
    --hand "go ho so eu        e7 e8  ga gz" \
    --hand "go ho so           e7 e8  ga gz gk" \
    --hand "go ho              e7 e8  ga gz gk g9" \
    --hand "go                 e7 e8  ga gz gk g9 g8" \
    --hand "                   e7 e8  ga gz gk g9 g8 g7" \
    \
    --hand "ho so eu gu hu  e7 e8 e9" \
    --hand "ho so eu gu     e7 e8 e9  ga" \
    --hand "ho so eu        e7 e8 e9  ga gz" \
    --hand "ho so           e7 e8 e9  ga gz gk" \
    --hand "ho              e7 e8 e9  ga gz gk g9" \
    --hand "                e7 e8 e9  ga gz gk g9 g8" \
    \
    --hand "so eu gu hu  e7 e8 e9 ek" \
    --hand "so eu gu     e7 e8 e9 ek  ga" \
    --hand "so eu        e7 e8 e9 ek  ga gz" \
    --hand "so           e7 e8 e9 ek  ga gz gk" \
    --hand "             e7 e8 e9 ek  ga gz gk g9" \
    --inspect "let vecn_eichel = ctx.eichel().extract(1); if vecn_eichel.contains(0) {0<ctx.trumpf(vecn_eichel.index_of(0))} else {false}"

echo
echo "Eichel-Rufspiel von 1 (mit 0) (mind. 4 Trumpf, Eichel nicht längste Farbe) - ist Gegner Eichel frei und hat Trumpf?"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "eichel rufspiel von 1" --constrain-hands "ctx.trumpf(1)>=4 && (ctx.gras(1)==0 || ctx.eichel(1)<=ctx.gras(1)) && (ctx.schelln(1)==0 || ctx.eichel(1)<=ctx.schelln(1))" \
    --hand "go ho so eu gu hu su  ea" \
    --hand "go ho so eu gu hu     ea  ga" \
    --hand "go ho so eu gu        ea  ga gz" \
    --hand "go ho so eu           ea  ga gz gk" \
    --hand "go ho so              ea  ga gz gk g9" \
    --hand "go ho                 ea  ga gz gk g9 g8" \
    --hand "go                    ea  ga gz gk g9 g8 g7" \
    --hand "                      ea  ga gz gk g9 g8 g7  sa" \
    \
    --hand "ho so eu gu hu su  ea e7" \
    --hand "ho so eu gu hu     ea e7  ga" \
    --hand "ho so eu gu        ea e7  ga gz" \
    --hand "ho so eu           ea e7  ga gz gk" \
    --hand "ho so              ea e7  ga gz gk g9" \
    --hand "ho                 ea e7  ga gz gk g9 g8" \
    --hand "                   ea e7  ga gz gk g9 g8 g7" \
    \
    --hand "ho so eu gu hu  ea e7 e8" \
    --hand "ho so eu gu     ea e7 e8  ga" \
    --hand "ho so eu        ea e7 e8  ga gz" \
    --hand "ho so           ea e7 e8  ga gz gk" \
    --hand "ho              ea e7 e8  ga gz gk g9" \
    --hand "                ea e7 e8  ga gz gk g9 g8" \
    \
    --hand "so eu gu hu  ea e7 e8 e9" \
    --hand "so eu gu     ea e7 e8 e9  ga" \
    --hand "so eu        ea e7 e8 e9  ga gz" \
    --hand "so           ea e7 e8 e9  ga gz gk" \
    --hand "             ea e7 e8 e9  ga gz gk g9" \
    \
    --hand "eu gu hu  ea e7 e8 e9 ek" \
    --hand "eu gu     ea e7 e8 e9 ek  ga" \
    --hand "eu        ea e7 e8 e9 ek  ga gz" \
    --hand "          ea e7 e8 e9 ek  ga gz gk" \
    --inspect "/*0 and 1 are primary party.*/(ctx.eichel(2)==0 && 0<ctx.trumpf(2)) || (ctx.eichel(3)==0 && 0<ctx.trumpf(3))"
