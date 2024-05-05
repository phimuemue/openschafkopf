read -p "Enter username: " username
read -p "Enter password: " password

curl -u "$username:$password" https://www.sauspiel.de/spiele/100000000 | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/100000001.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395181267.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395253010.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395330588.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395278368.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395429053.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395446665.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395466089.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395465424.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395464395.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395435897.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395329693.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395329350.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395427746.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395466433.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395330178.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395330500.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395329502.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1395329445.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1394803257.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1394368336.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1394812376.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1392731627.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1391236115.json | ./target/release/openschafkopf parse --raw
curl -u "$username:$password" https://www.sauspiel.de/spiele/1390243503.json | ./target/release/openschafkopf parse --raw
