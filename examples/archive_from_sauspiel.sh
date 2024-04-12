read -p "Enter username: " username

curl -u "$username" https://www.sauspiel.de/spiele/100000000 | ./target/release/openschafkopf parse --raw

