import sys

an_initial = [0,0,0,0]

for str_line in sys.stdin.readlines():
    an_line = [int(str_split) for str_split in str_line.split()][2:-1]
    an_diff = [an_line[i]-an_initial[i] for i in range(4)]
    print(",".join([str(n_diff) for n_diff in an_diff]))
    an_initial = an_line
