N_SIMULATE_HANDS=10000

./target/release/openschafkopf hand-stats --rules "ramsch" --simulate-hands $N_SIMULATE_HANDS \
    --hand "sa sz sk so su s9 s8 s7" \
    --hand "sa sz sk so su s9" \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,0)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,1)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,2)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,3)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,4)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,5)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,6)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,7)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,8)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,9)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,10)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,11)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,12)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,13)' \
    --inspect 'import "examples/kartenverteilung_auf_mitspieler.rhai" as ext; ext::kartenverteilung(ctx,14)' \
