./target/release/openschafkopf websocket &
sleep 2 # time to start openschafkopf

# try to open for four players, each requiring to enter a name
sensible-browser lib/tools/site/site.html
sensible-browser lib/tools/site/site.html
sensible-browser lib/tools/site/site.html
sensible-browser lib/tools/site/site.html

echo "Browser should have opened for four players. Beware that openschafkopf is running as a background process."
