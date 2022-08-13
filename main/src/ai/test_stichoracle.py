#!/usr/bin/python3

import sys
import re
import itertools

str_re_card = "[eghs][789kzuoka]"
str_re_cards = " *(%s *)*"%(str_re_card)
str_re_hand = "(?P<veccard_0>%s),(?P<veccard_1>%s),(?P<veccard_2>%s),(?P<veccard_3>%s),(?P<veccard_4>%s)"%(str_re_cards, str_re_cards, str_re_cards, str_re_cards, str_re_cards)
re_hand = re.compile(str_re_hand, flags=re.IGNORECASE)

assert(len(sys.argv)==2)
vecstr_hand = sys.argv[1].upper().split("|")
vecgroupdict = [re_hand.match(str_hand).groupdict() for str_hand in vecstr_hand]

vecvecstr_cards = [[groupdict[str_key] for str_key in ["veccard_0", "veccard_1", "veccard_2", "veccard_3", "veccard_4"]] for groupdict in vecgroupdict]

vecvecvecstr_card = [[[str_card for str_card in str_cards.split(" ") if len(str_card)==2] for str_cards in vecstr_cards] for vecstr_cards in vecvecstr_cards]
print("[")
for vecvecstr_card in vecvecvecstr_card:
    print("    &[%s],"%(", ".join(sum(vecvecstr_card, []))))
print("],")

print("&[],")

def get_trumpf_farbe_or_all(vecvect, i):
    return vecvect[i] or sum(vecvect,[])

print("&[")
for vecvecstr_hands in [[get_trumpf_farbe_or_all(vecvecstr_card, i) for vecvecstr_card in vecvecvecstr_card] for i in range(0,5) if vecvecvecstr_card[0][i]]:
    for prod in itertools.product(*vecvecstr_hands):
        print("    [%s],"%(", ".join(prod).upper()))
print("],")


# ./test_stichoracle.py 'eo go ho gz g7,ea ez,,hk,| so eu hu ga gk,,,ha,sa s9 | gu su g9 g8,e9,,h8,sk s7 | ,ek e8 e7,,hz h9 h7, sz s8'
