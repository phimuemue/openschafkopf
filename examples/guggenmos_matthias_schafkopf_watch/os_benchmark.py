import os
import subprocess
from datetime import datetime

import numpy as np
import pandas as pd

# IMPORTANT: replace with path to openschafkopf binary
PATH_OS = '/opt/openschafkopf/openschafkopf'

multiproc = False  # if True, use one thread per optimizer; if False, use a single thread
test_mode = True  # if True, analyze only the first test_mode_ngames games
test_mode_ngames = 10
enable_verbose = True

if enable_verbose:
    import json

PATH_DATA = 'data/data.parquet'

STRATEGY = 'maxselfishmin'
nplayers = 4
nrounds_lange = 8
nrounds_kurze = 6

# commented optimizers are computationally extremely
# expensive for the first few Ausspiele
optimizers = dict(
    # none='',
    # ab='--abprune',
    ssc='--snapshotcache',
    # ab4='--abprune --branching equiv4',
    # ab5='--abprune --branching equiv5',
    # ab6='--abprune --branching equiv6',
    abo='--abprune --branching oracle',
    ssc4='--snapshotcache --branching equiv4',
    ssc5='--snapshotcache --branching equiv5',
    ssc6='--snapshotcache --branching equiv6',
    ssco='--snapshotcache --branching oracle'
)
noptimizers = len(optimizers)

FARBEN = {0: 'Schellen', 1: 'Herz', 2: 'Gras', 3: 'Eichel'}
FARBEN_SAUSPIEL = {0: 'Hundsgfickte', 2: 'Blaue', 3: 'Alte'}
GAME_TYPES = {0: 'Ramsch', 1: 'Sauspiel', 2: 'Farbwenz', 3: 'Geier', 4: 'Wenz', 5: 'Solo'}
KARTEN = {
    0: 'S7', 1: 'H7', 2: 'G7', 3: 'E7',
    4: 'S8', 5: 'H8',6: 'G8', 7: 'E8',
    8: 'S9', 9: 'H9', 10: 'G9', 11: 'E9',
    12: 'SK', 13: 'HK', 14: 'GK', 15: 'EK',
    16: 'SZ', 17: 'HZ', 18: 'GZ', 19: 'EZ',
    20: 'SA', 21: 'HA', 22: 'GA', 23: 'EA',
    24: 'SU', 25: 'HU', 26: 'GU', 27: 'EU',
    28: 'SO', 29: 'HO', 30: 'GO', 31: 'EO'
}


def os_execution_time(gameid, hands, cards, ausspieler, gametype, farbe, position_spieler, optimizer, verbose=False):
    """ Compute execution time for Schafkopf Rufspiele and
        Solospiele (but not Zam'gworfen or Ramsch).

    Args:
        gameid (int): sauspiel.de game ID
        hands (List[List[int]]): hand of each player (nplayers x ncards)
        cards (List[List[int]]): cards played in each round (nrounds x nplayers)
        ausspieler (List[int]): position of the Ausspieler (0-based)
        gametype (int): game type (-1=Zam'gworfen 0=Ramsch 1=Sauspiel 2=Farbwenz 3=Geier 4=Wenz 5=Solo)
        farbe (int): Ruffarbe or Trumpffarbe (0=Schellen 1=Herz 2=Gras 3=Eichel)
        position_spieler (int): position of the Rufspieler or Solospieler (0-based)
        optimizer (str): key for the optimizer string (e.g. 'abo')
        verbose (bool): if True, print additional information

    Returns:
        result (List): returns a list with 2 elements in this order:
            - overall execution time (float)
            - execution time for each Ausspiel (List[float])

    """
    if gametype in (-1, 0):
        if verbose:
            print(f"\n[{gameid}] Ramsch or Zam'gworfen are not supported.")
        return pd.NA
    else:
        if gametype == 1:
            rule = f'Sauspiel auf die {FARBEN_SAUSPIEL[farbe]} von {position_spieler:.0f}'
        elif gametype in (2, 5):
            rule = f'{FARBEN[farbe]}-{GAME_TYPES[gametype]} von {position_spieler:.0f}'
        else:
            rule = f'{GAME_TYPES[gametype]} von {position_spieler:.0f}'
        if verbose:
            print(f'\n[{gameid}] {rule}')

        nrounds = len(cards)
        command_base = f'{PATH_OS} suggest-card --points --strategy {STRATEGY} --json --rules "{rule}" {optimizer}'

        hands1d = np.array([[KARTEN[c] for c in hand] for hand in hands]).flatten()

        execution_time = []
        cards_table = ''
        for r in range(nrounds - 1):
            if verbose:
                print(f'\t[{datetime.now().strftime('%H:%M:%S')}] Round {r + 1} / {len(cards)}')
                print(f'\t\tAusspieler: {ausspieler[r]}')
            cards_round = [KARTEN[c] for c in cards[r]]
            position = ausspieler[r]
            for s in range(nplayers):
                cards_hand = ' '.join(hands1d)
                command_ausspiel = f'--cards-on-table "{cards_table}" --hand "{cards_hand}"'
                command = f'{command_base} {command_ausspiel}'
                if verbose:
                    print(f'\t\t____________')
                    print(f'\t\t{command_ausspiel}')

                start_time = datetime.now()
                os_result_json = subprocess.check_output(command, shell=True, text=True)
                execution_time += [int(np.round(1000*(datetime.now() - start_time).total_seconds()))]
                if verbose:
                    os_result = json.loads(os_result_json)['vectableline'][:-1]
                    points_predicted = {res['ostr_header']:
                        res['perminmaxstrategyvecpayout_histogram'][STRATEGY][0][0][0] for res in os_result
                    }
                    print(f'\t\tAusspiel {s + 1}: {cards_round[s]} ({points_predicted} points, '
                          f'max={max(points_predicted.values())})')
                cards_table += f' {cards_round[s]}'
                hands1d = np.setdiff1d(hands1d, cards_round[s], assume_unique=True)
                position = (position + 1) % nplayers
                if verbose:
                    print(f'\t\tExecution time: {execution_time[-1]:0f} ms')

        result = [int(sum(execution_time)), execution_time]
        return result


def loop(f):
    k, optimizer = list(optimizers.items())[f]
    print(f'\n\n Optimizer {f + 1} / {noptimizers}: {k}={optimizer}\n')
    df = pd.read_parquet(PATH_DATA)
    if test_mode:
        df = df[:test_mode_ngames].copy()
    cols_new = ['execution_time', 'execution_times']

    # to get a progress bar use
    # from tqdm import tqdm
    # tqdm.pandas()
    # df.progress_apply(...)
    df[cols_new] = df.apply(lambda x: os_execution_time(
        x['id'],
        [x[f'player{i}_cards'] for i in range(nplayers)],
        [x[f'round{i}_cards'] for i in range(nrounds_kurze if x['rule_kurze'] else nrounds_lange)],
        [x[f'round{i}_ausspieler_id'] for i in range(nrounds_kurze if x['rule_kurze'] else nrounds_lange)],
        x['game_type'],
        x['farbe'],
        x['spieler_id'],
        optimizer, verbose=enable_verbose
    ), axis=1, result_type='expand')

    path_save = f'data/{k}{"_testmode" if test_mode else ""}.parquet'
    df[['id', 'rule_kurze', 'game_type'] + cols_new].to_parquet(path_save)
    print(f'\t[{datetime.now().strftime('%H:%M:%S')}] Wrote file to {path_save}')


if __name__ == '__main__':

    # Change to the script directory
    os.chdir(os.path.dirname(os.path.abspath(__file__)))

    if multiproc:
        from multiprocessing.pool import Pool
        with Pool(noptimizers) as pool:
            pool.map(loop, range(noptimizers))
    else:
        for i in range(noptimizers):
            loop(i)

    print(f'[{datetime.now().strftime('%H:%M:%S')}] Finished')
