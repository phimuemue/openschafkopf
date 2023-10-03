# done with ChatGPT help

# Set the output file format and name (SVG)
set terminal svg enhanced size 800,300

# Set the title of the plot

# Set the labels for the x and y axes

set border 2

set xtics nomirror
set ytics nomirror

# Specify the input data files and their column mappings
set datafile separator ","
file_baseline = "baseline/csv.csv"
file_cheat_ab_stich_3 = "cheat_ab_stich_3/csv.csv"

set xrange [0:600]

# set key bottom left Left reverse
unset key

# Define the plot style for each dataset (lines)
# Raw payout:
set output "output_money.svg"
set yrange [-200:250]
set multiplot layout 1,2
set title "Raw payout normal"
plot \
    file_baseline using 1:2 with lines lt rgb "grey" notitle,\
    file_baseline using 1:3 with lines lt rgb "grey" notitle,\
    file_baseline using 1:4 with lines lt rgb "grey" title "Links/Oben/Rechts", \
    file_baseline using 1:5 with lines lt rgb "red" title "Gast", \

set title "Raw payout with cheating"
plot\
    file_cheat_ab_stich_3 using 1:2 with lines lc rgb "grey" notitle,\
    file_cheat_ab_stich_3 using 1:3 with lines lc rgb "grey" notitle,\
    file_cheat_ab_stich_3 using 1:4 with lines lc rgb "grey" title "Links/Oben/Rechts", \
    file_cheat_ab_stich_3 using 1:5 with lines lc rgb "red" title "Gast",\

unset multiplot

set output "output_games_won_minus_lost.svg"
set yrange [-50:100]
set multiplot layout 1,2
set title "Games won minus games lost normal"
plot \
    file_baseline using 1:6 with lines lt rgb "grey" notitle,\
    file_baseline using 1:7 with lines lt rgb "grey" notitle,\
    file_baseline using 1:8 with lines lt rgb "grey" title "Links/Oben/Rechts", \
    file_baseline using 1:9 with lines lt rgb "red" title "Gast", \

set title "Games won minus games lost with cheating"
plot\
    file_cheat_ab_stich_3 using 1:6 with lines lc rgb "grey" notitle,\
    file_cheat_ab_stich_3 using 1:7 with lines lc rgb "grey" notitle,\
    file_cheat_ab_stich_3 using 1:8 with lines lc rgb "grey" title "Links/Oben/Rechts", \
    file_cheat_ab_stich_3 using 1:9 with lines lc rgb "red" title "Gast",\

unset multiplot
