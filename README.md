# OpenSchafkopf

[![Build Status](https://travis-ci.com/phimuemue/openschafkopf.svg?token=p5VJrpqP4RgYm9XasCJN&branch=master)](https://travis-ci.com/phimuemue/openschafkopf)
[![codecov](https://codecov.io/gh/phimuemue/openschafkopf/branch/master/graph/badge.svg)](https://codecov.io/gh/phimuemue/openschafkopf)

This is a work in progress, aiming to model the game Schafkopf in a robust and extensible manner. Moreover, it aims to provide an AI playing the game. As of now, it does *not* provide any fancy user interface, but instead relies on the command line.

[![asciicast](https://asciinema.org/a/q8IiXdkHZAnRvkC8yL4eOt6Gf.png)](https://asciinema.org/a/q8IiXdkHZAnRvkC8yL4eOt6Gf)

## Getting started

You need the rust programming language to build OpenSchafkopf (see https://www.rust-lang.org/en-US/install.html).

```
curl https://sh.rustup.rs -sSf | sh
git clone https://github.com/openschafkopf/openschafkopf.git
cd openschafkopf
cargo build --release
./target/release/openschafkopf cli
```

## Supported variants

* Rufspiel
* Solo
* Wenz
* Farbwenz
* Geier
* Farbgeier
* Tout/Sie
* Bettel
* Ramsch
* Stock
* Schneider/Schwarz
* Laufende (adjustible per variant)
* Stoss
* Doppeln
* Steigern
* Kurze Karte/Lange Karte
