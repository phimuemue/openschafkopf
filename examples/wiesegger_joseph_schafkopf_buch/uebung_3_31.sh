# Surprise: This finds another error from the one mentioned in the book

./target/release/openschafkopf analyze examples/wiesegger_joseph_schafkopf_buch/uebung_3_31.txt --max-remaining-cards 5 --include-no-findings --simulate-all-hands
echo "Analysis found mistake in Stich 6. Analysis if other cards are unknown to player:"
./target/release/openschafkopf suggest-card --rules "Rufspiel mit der Gras-Sau von 0" --cards-on-table "HO HK H7 GO  GZ G9 GA G8  EU H8 GU HU  S7 S8 SK SA  EA E9 H9 EK  GK E8" --hand "SO EZ SU" --branching "9,9" --simulate-hands all
