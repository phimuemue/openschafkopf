*Caveat:* This is a work-in-progress. I'm not sure if I got my acquaintance right in all aspects.

# Intro

An acquaintance's student evaluated the winning probability of a Wenz Tout with EU HU SU SA SZ SK.

The results were:

* For position 0: 100% (trivial: play EU to collect GU and afterwards play cards in any order)
* For position 1: 81.64%
* For position 2: 86.59%
* For position 3: 90.29%

# My take for position 1 or 2

TODO: In this case, Schelln is dangerous and leads to a direct loss if another player has no Schelln but holds Gras-Unter. In addition to that, other Farben may lead to loss if the player's Herz- or Schelln-Unter can be topped by another player.

# My take for position 3

When the Wenz Tout is played in the last position, the game is only lost if the opening player chooses Schelln, and player 1 or 2 has no schelln *and* holds Gras-Unter.

We can simulate this by fixing the hand of player 3 to EU HU SU SA SZ SK, randomly distributing the other cards onto players 0 to 2.

If I got it right, the student assumed that player 0 plays a card of their longest Farben without an Ass. If there are multiple applicable Farben, player 0 chooses randomly between them.

This is implemented by `position_3.sh`. Running it resulted in this:

```
[[Eichel, Gras, Herz], 0, 0, 0] 8481 (0.85%)
[[Eichel, Gras, Herz], 1, 0, 0] 3123 (0.31%)
[[Eichel, Gras, Schelln], 0, 0.3333333333333333, 0] 11560 (1.16%)
[[Eichel, Gras, Schelln], 1, 0.3333333333333333, 0.3333333333333333] 7546 (0.75%)
[[Eichel, Gras], 0, 0, 0] 36407 (3.64%)
[[Eichel, Gras], 1, 0, 0] 13685 (1.37%)
[[Eichel, Herz, Schelln], 0, 0.3333333333333333, 0] 11495 (1.15%)
[[Eichel, Herz, Schelln], 1, 0.3333333333333333, 0.3333333333333333] 7555 (0.76%)
[[Eichel, Herz], 0, 0, 0] 36462 (3.65%)
[[Eichel, Herz], 1, 0, 0] 13925 (1.39%)
[[Eichel, Schelln], 0, 0.5, 0] 18897 (1.89%)
[[Eichel, Schelln], 1, 0.5, 0.5] 18986 (1.90%)
[[Eichel], 0, 0, 0] 132894 (13.29%)
[[Eichel], 1, 0, 0] 49807 (4.98%)
[[Gras, Herz, Schelln], 0, 0.3333333333333333, 0] 11626 (1.16%)
[[Gras, Herz, Schelln], 1, 0.3333333333333333, 0.3333333333333333] 7691 (0.77%)
[[Gras, Herz], 0, 0, 0] 36479 (3.65%)
[[Gras, Herz], 1, 0, 0] 13880 (1.39%)
[[Gras, Schelln], 0, 0.5, 0] 18897 (1.89%)
[[Gras, Schelln], 1, 0.5, 0.5] 19277 (1.93%)
[[Gras], 0, 0, 0] 134236 (13.42%)
[[Gras], 1, 0, 0] 49748 (4.97%)
[[Herz, Schelln], 0, 0.5, 0] 18749 (1.87%)
[[Herz, Schelln], 1, 0.5, 0.5] 19069 (1.91%)
[[Herz], 0, 0, 0] 133558 (13.36%)
[[Herz], 1, 0, 0] 49876 (4.99%)
[[Schelln], 0, 1, 0] 37317 (3.73%)
[[Schelln], 1, 1, 1] 43091 (4.31%)
[[], 0, 0, 0] 29331 (2.93%)
[[], 1, 0, 0] 6352 (0.64%)
-----
⌀ [⊥, 0.3236, 0.1565, 0.0794]
```

The very last number (`0.0794`) indicates that the probability that player 0 plays schelln and another player has no Schelln *and* holds the Gras-Unter is just below 8%. Inverting this leads to a winning probability of ~92% - close to the student's result, but not exactly equal.
