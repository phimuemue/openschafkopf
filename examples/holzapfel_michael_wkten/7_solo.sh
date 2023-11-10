set -e

echo "https://www.michael-holzapfel.de/schk/ws7-S/ws-Solo.htm - Relative Haeufigkeiten beim Solo"

N_SIMULATE_HANDS=100000

echo ""
echo "1./2. Fall: 2 Ober, 2 Unter, 2 Herz"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "herz-solo von 0" --hand "eo ho eu hu hz hk" \
    --inspect "
/*4 Trumpf*/ ctx.trumpf().extract(1).contains(4)" \
    --inspect '
/*4 Trumpf, dabei 2 Ober und 1 oder 2 Unter*/ import "examples/holzapfel_michael_wkten/7_solo_gegner_trumpf_ober_unter.rhai" as ext; ext::any_has_trumpf_ober_unter(ctx, 4, 2, 1) || ext::any_has_trumpf_ober_unter(ctx, 4, 2, 2)' \
    --inspect "
/*5 Trumpf*/ ctx.trumpf().extract(1).contains(5)" \

echo ""
echo "3. Fall: Eichel-Ober, Eichel-, Herz- und Schelln-Unter, 2 Herz"
./target/release/openschafkopf hand-stats --simulate-hands $N_SIMULATE_HANDS  --rules "herz-solo von 0" --hand "eo eu hu su hz h9" \
    --inspect '
/*(B1) 3 Trumpf, exakt 1 Ober*/ import "examples/holzapfel_michael_wkten/7_solo_gegner_trumpf_ober_unter.rhai" as ext; ext::any_has_trumpf_ober(ctx, 3, 1)' \
    --inspect '
/*(B2) 3 Trumpf, exakt 2 Ober*/ import "examples/holzapfel_michael_wkten/7_solo_gegner_trumpf_ober_unter.rhai" as ext; ext::any_has_trumpf_ober(ctx, 3, 2)' \
    --inspect '
/*(B3) 3 Trumpf, exakt 3 Ober*/ import "examples/holzapfel_michael_wkten/7_solo_gegner_trumpf_ober_unter.rhai" as ext; ext::any_has_trumpf_ober(ctx, 3, 3)' \
    --inspect '
/*(B) 3 Trumpf, dabei mindestens 1 Ober*/ import "examples/holzapfel_michael_wkten/7_solo_gegner_trumpf_ober_unter.rhai" as ext; ext::any_has_trumpf_ober(ctx, 3, 1) || ext::any_has_trumpf_ober(ctx, 3, 2) || ext::any_has_trumpf_ober(ctx, 3, 3)' \
    --inspect '
/*(C) 4 Trumpf, dabei mindestens 2 Ober*/ import "examples/holzapfel_michael_wkten/7_solo_gegner_trumpf_ober_unter.rhai" as ext; ext::any_has_trumpf_ober(ctx, 4, 2) || ext::any_has_trumpf_ober(ctx, 4, 3)' \
    --inspect '
/*(D) 5 Trumpf*/ctx.trumpf().extract(1).contains(5)' \
    --inspect '
/*(Speziell) 5 Trumpf, 3 Ober*/ctx.trumpf().extract(1).contains(5)&&ctx.ober().extract(1).contains(3)' \
    --inspect '
/*(Speziell) 3 Ober*/ctx.ober().extract(1).contains(3)' \

echo ""
echo "Bemerkung: Das Herz-Solo EO EU HU SU HZ H9 als Ausspieler zu gewinnen, ist unwahrscheinlich"
./target/release/openschafkopf suggest-card --hand "eo eu hu su hz h9" --rules "herz solo von 0" --simulate-hands 100 --branching equiv5 --points --snapshotcache
