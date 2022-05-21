# OpenSchafkopf

This is a work in progress, aiming to model the game Schafkopf in a robust and extensible manner. Moreover, it aims to provide an AI playing the game. As of now, it does *not* provide any fancy user interface, but instead relies on the command line.

## Getting started

Build Requirements:
* rust programming language (https://www.rust-lang.org/en-US/install.html)
* inkscape (https://inkscape.org/)
* less (http://lesscss.org/)

```
git clone https://github.com/phimuemue/openschafkopf.git
cd openschafkopf
cargo build --release
./target/release/openschafkopf -h

schafkopf 

USAGE:
    openschafkopf [SUBCOMMAND]

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    cli             Simulate players to play against
    websocket       Play in the browser
    analyze         Analyze played games and spot suboptimal decisions
    suggest-card    Suggest a card to play given the game so far
    rank-rules      Estimate strength of own hand
    hand-stats      Statistics about hands that could be dealt.
    dl              Download played games from Sauspiel
    parse           Parse a game into a simple format
    webext          Backend of a web-extension suggesting a card for a given game state
    help            Print this message or the help of the given subcommand(s)
```

## Supported variants

* Kurze Karte/Lange Karte
* Rufspiel
* Solo/Wenz/Farbwenz/Geier/Farbgeier
* Schneider/Schwarz/Laufende
* Tout/Sie
* Stoss/Doppeln
* Ramsch
* Bettel
* Stock
* Steigern
