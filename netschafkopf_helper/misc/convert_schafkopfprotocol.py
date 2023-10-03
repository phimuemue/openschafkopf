# done with ChatGPT help

import sys
import re

# Check if the correct number of command line arguments are provided
if len(sys.argv) < 2:
    print("Usage: python program.py file1.txt file2.txt ...")
    sys.exit(1)

def extract_numbers_from_string(input_string):
    # Define a regular expression pattern to match the desired format
    pattern = r'^\s*(\d+)\s+(\d+)\s+([\+\-]\d+)\s+([\+\-]\d+)\s+([\+\-]\d+)\s+([\+\-]\d+)\s+(\d+)\s*$'
    # Try to match the input string with the pattern
    match = re.match(pattern, input_string)
    # Check if the string matches the pattern
    assert(match)
    # Extract and convert the matched numbers to integers
    numbers = [int(match.group(i)) for i in range(1, 8)]
    return numbers

def offset_vectors(vectors, offset_vector):
    # Ensure that the offset_vector has 4 elements
    assert len(offset_vector) == 4, "Offset vector must have 4 elements"
    offset_result = []
    for vector in vectors:
        # Ensure that each input vector has 4 elements
        assert len(vector) == 4, "Input vectors must have 4 elements"
        offset_result.append([vector[i] + offset_vector[i] for i in range(4)])
    return offset_result
    
# Function to extract lines following "spiel preis"
def extract_lines_with_spiel_preis(file_path, offset_vector):
    vecline = []
    spiel_preis_pattern = re.compile(r'^Spiel\s+Preis', re.IGNORECASE)
    try:
        with open(file_path, 'r', encoding='latin-1') as file:
            lines = file.readlines()
            spiel_preis_found = False
            for line in lines:
                if spiel_preis_found:
                    vecline.append(extract_numbers_from_string(line.strip())[2:6])
                    spiel_preis_found = False
                if spiel_preis_pattern.match(line):
                    spiel_preis_found = True
    except FileNotFoundError:
        print(f"File not found: {file_path}")
    return offset_vectors(vecline, offset_vector)

# Iterate through the command-line arguments and process each file
vecan_account = [[0,0,0,0]]
for file_path in sys.argv[1:]:
    vecan_account.extend(extract_lines_with_spiel_preis(file_path, vecan_account[-1][:]))
#print(vecan_account)

#print("----")

vecan_won_accumulated = [[0,0,0,0]]
an_account_prev = vecan_account[0]
for an_account in vecan_account[1:]:
    an_payout = [an_account[i]-an_account_prev[i] for i in range(4)]
    vecan_won_accumulated.append([vecan_won_accumulated[-1][i] + (1 if an_payout[i]>0 else -1) for i in range(4)])
    an_account_prev = an_account
#print(vecan_won_accumulated)

assert(len(vecan_account)==len(vecan_won_accumulated))

for (i, (an_account, an_won_accumulated)) in enumerate(zip(vecan_account, vecan_won_accumulated)):
    print(",".join(str(n_number) for n_number in ([i]+ an_account+an_won_accumulated)))
