# OpenSchafkopf

This software allows to play and analyze the game [Schafkopf](https://en.wikipedia.org/wiki/Schafkopf). It supports the following variants (see [rulesets](https://github.com/phimuemue/openschafkopf/tree/main/rulesets) for examples):

* Rules: Rufspiel, Solo/Wenz/Farbwenz/Geier/Farbgeier (including Tout/Sie), Bettel, Ramsch
* Expensifiers: Schneider/Schwarz, Laufende, Stoss, Doppeln
* Misc: Kurze/lange Karte, Stock, Steigern


## Building

Requirements:
* [rust](https://www.rust-lang.org/en-US/install.html) stable
* [inkscape](https://inkscape.org/) (tested with 1.0.2)
* [less](http://lesscss.org/) (tested with 3.9.0)

```
git clone https://github.com/phimuemue/openschafkopf.git
cd openschafkopf
cargo build --release
```

## Getting started

### Examples

The repository contains some [examples](https://github.com/phimuemue/openschafkopf/tree/main/examples) that show how the program can be used. They are meant to be run from the root folder.

### More details

```
./target/release/openschafkopf -h

schafkopf 

USAGE:
    openschafkopf [SUBCOMMAND]

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    cli             Play in command line
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
