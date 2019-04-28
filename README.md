# OpenSchafkopf

[![Build Status](https://travis-ci.com/phimuemue/openschafkopf.svg?token=p5VJrpqP4RgYm9XasCJN&branch=master)](https://travis-ci.com/phimuemue/openschafkopf)
[![codecov](https://codecov.io/gh/phimuemue/openschafkopf/branch/master/graph/badge.svg)](https://codecov.io/gh/phimuemue/openschafkopf)

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

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    analyze       
    cli           
    help          Prints this message or the help of the given subcommand(s)
    rank-rules    
```

## Features

* Estimate strength of own hand.
* Simulate players to play against.
* Analyze played games and spot suboptimal decisions.

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
