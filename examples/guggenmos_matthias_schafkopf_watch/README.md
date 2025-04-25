**Author:** Matthias Guggenmos, 2025

# Openschafkopf optimization for schafkopf.watch

[schafkopf.watch](https://schafkopf.watch) is a website that provides numerous statistical analyses of the Bavarian card game Schafkopf. As one goal is the measurement of player strength, the project **openschafkopf** was assessed as a possible means to compute the optimality of played cards. The code provided here is concerned with the optimization of game tree exploration which is necessary to analyse several million games.

## Data `data/data.parquet`

Analyses are based on 25,000 games stored in [Apache Parquet](https://parquet.apache.org/) format, an efficient column-oriented data file format well handed by the [Pandas library](https://pandas.pydata.org/) with either the [fastparquet](https://github.com/dask/fastparquet) or the [pyarrow](https://arrow.apache.org/) backend. The Schafkopf games were provided by [sauspiel.de](sauspiel.de) in the context of a scientific cooperation.

## Benchmarking script `os_benachmark.py`

This code assesses optimal openschafkopf strategies to speed up game tree exploration for the command `openschafkopf suggest-card`. The following settings were tested:

- `--abprune --branching oracle`
- `--snapshotcache`
- `--snapshotcache --branching equiv4`
- `--snapshotcache --branching equiv5`
- `--snapshotcache --branching equiv6`
- `--snapshotcache --branching oracle`

For this analysis, 25,000 games were assessed. The results are stored separately for each strategy:

- `data/abo.parquet`: `--abprune --branching oracle`
- `data/scc.parquet`: `--snapshotcache`
- `data/ssc4.parquet`: `--snapshotcache --branching equiv4`
- `data/ssc5.parquet`: `--snapshotcache --branching equiv5`
- `data/ssc6.parquet`: `--snapshotcache --branching equiv6`
- `data/ssco.parquet`: `--snapshotcache --branching oracle`

**Requirements:** Up-to-date versions of `pandas` and `numpy`. At the time of this investigation, `pandas` was used in version `2.2.2` and `numpy` in version `1.26.4`. 

## Visualization notebook `os_benachmark_visualize.ipynb`

This notebook analyzes and visualizes the results from `os_benachmark.py`. Execution times are assessed and visualized separately for Lange/Kurze Karte, game type (Rufspiel, Farbwenz, Geier, Wenz, Farbsolo) and Ausspiel number. 

**Requirements:** Up-to-date versions of `pandas`, `numpy`, `matplotlib` and `scipy`. At the time of this investigation, `pandas` was used in version `2.2.2`, `numpy` in version `1.26.4`, `matplotlib` in version `3.9.0` and `scipy` in version `1.13.1`.

## Results

Based on the present analyses, a suggested set of strategies for `openschafkopf suggest-card` is as follows:


| Kurze Karte    |                   |   |   |
|----------------|-------------------|---|---|
| ***Rufspiel*** | Ausspiel 1-7 abo  | Rest ssc |   |
| ***Farbwenz*** | Ausspiel 1-4 ssco | Rest ssc |   |
| ***Geier***    | ssc               |   |   |
| ***Wenz***     | ssc               |   |   |
| ***Farbsolo*** | Ausspiel 1-2 ssco | Ausspiel 3-4 abo | Rest ssc |

| Lange Karte    |                    |   |   |
|----------------|--------------------|---|---|
| ***Rufspiel*** | Ausspiel 1-16 abo  | Rest ssc |   |
| ***Farbwenz*** | Ausspiel 1-7 ssco  | Ausspiel 8-16 abo |  Rest ssc |
| ***Geier***    | Ausspiel 1-9 ssco  | Ausspiel 10-15 abo |  Rest ssc |
| ***Wenz***     | Ausspiel 1-10 ssco | Ausspiel 11-16 abo |  Rest ssc |
| ***Farbsolo*** | Ausspiel 1-9 ssco  | Ausspiel 10-16 abo | Rest ssc |

**Abbreviations:** abo=`--abprune --branching oracle`, scc=`--snapshotcache`, ssc4=`--snapshotcache --branching equiv4`, ssc5=`--snapshotcache --branching equiv5`, ssc6=`--snapshotcache --branching equiv6`, ssco=`--snapshotcache --branching oracle`

**Limitations:**

- Ramsch was not assessed.
- Computing resources are not exactly identical for all strategies (but likely minor effect).
- There is residual uncertainty with respect to the exact Ausspiel number on which a strategy switch should occur, but this is likewise a neglegible effect.