use crate::game_analysis::*;
use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use itertools::Itertools;

pub fn parse_rule_description(
    str_rules_with_player: &str,
    (n_tarif_extra, n_tarif_ruf, n_tarif_solo): (isize, isize, isize),
    fn_player_to_epi: impl Fn(&str)->Result<EPlayerIndex, Error>,
) -> Result<Box<dyn TRules>, Error> {
    use crate::rules::rulesrufspiel::*;
    use crate::rules::rulessolo::*;
    use crate::rules::rulesbettel::*;
    use crate::rules::rulesramsch::*;
    use crate::rules::payoutdecider::*;
    let (str_rules, ostr_epi_active) = {
        if let Some((str_rules, str_epi_active)) = str_rules_with_player.split(" von ").collect_tuple() {
            (str_rules, Some(str_epi_active))
        } else if let Some((str_epi_active, str_rules)) = str_rules_with_player.split(" spielt ").collect_tuple() {
            (str_rules, Some(str_epi_active))
        } else {
            (str_rules_with_player, None)
        }
    };
    let str_rules = str_rules.to_lowercase();
    let get_epi_active = || -> Result<EPlayerIndex, Error> {
        ostr_epi_active
            .ok_or_else(|| format_err!("Cannot determine active player: {}", str_rules_with_player))
            .and_then(&fn_player_to_epi)
    };
    // Regarding laufende:
    // https://www.sauspiel.de/hilfe#71-beim-farbwenz-wurden-meine-laufende-nicht-berechnet
    // https://www.schafkopfschule.de/index.php/regeln.html?file=files/inhalte/dokumente/Spielen/Regeln/Schafkopfregeln-Aktuell-29.3.2007.pdf (Section 4.2 Spielabrechnung)
    let str_rules_contains = |slcstr: &[&str]| slcstr.iter().any(|str| str_rules.contains(str));
    // determine oefarbe
    let oefarbe = match [
        (EFarbe::Eichel, &["eichel", "alt"] as &[&str]),
        (EFarbe::Gras, &["gras", "grün", "laub", "blatt", "blau"]),
        (EFarbe::Herz, &["herz", "rot"]),
        (EFarbe::Schelln, &["schell", "pump", "hundsgfickte"]),
    ].into_iter()
        .filter(|(_efarbe, slcstr_farbe)| str_rules_contains(slcstr_farbe))
        .exactly_one()
    {
        Ok((efarbe, _)) => Some(efarbe),
        Err(itefarbeslcstr) => {
            if 0==itefarbeslcstr.count() {
                None
            } else {
                return Err(format_err!("Could not clearly determine efarbe."))
            }
        },
    };
    let make_sololike = |esololike| {
        macro_rules! make_sololike_internal {($payoutdecider: ident) => {
            Ok(sololike(
                get_epi_active()?,
                oefarbe,
                esololike,
                $payoutdecider::default_payoutdecider(
                    /*n_payout_base*/n_tarif_solo,
                    /*n_payout_schneider_schwarz*/n_tarif_extra,
                    SLaufendeParams::new(
                        /*n_payout_single_player*/n_tarif_extra,
                        /*n_lauf_lbound*/if let Some(_efarbe)=oefarbe {3} else {2},
                    ),
                ),
            ).upcast_box())
        }}
        if str_rules_contains(&["tout"]) {
            make_sololike_internal!(SPayoutDeciderTout)
        } else {
            make_sololike_internal!(SPayoutDeciderPointBased)
        }
    };
    match [
        (&["rufspiel", "sauspiel", "mit der"] as &[&str], {
            match oefarbe {
                None => Err(format_err!("Rufspiel requires efarbe")),
                Some(efarbe) => match efarbe {
                    EFarbe::Herz => Err(format_err!("Rufspiel incompatible with EFarbe::Herz")),
                    EFarbe::Eichel | EFarbe::Gras | EFarbe::Schelln => {
                        get_epi_active().map(|epi| {
                            Box::new(SRulesRufspiel::new(
                                epi,
                                efarbe,
                                SPayoutDeciderParams::new(
                                    /*n_payout_base*/n_tarif_ruf,
                                    /*n_payout_schneider_schwarz*/n_tarif_extra,
                                    SLaufendeParams::new(
                                        /*n_payout_single_player*/n_tarif_extra,
                                        /*n_lauf_lbound*/3,
                                    ),
                                )
                            )) as Box<dyn TRules>
                        })
                    }
                }
            }
        }),
        (&["solo", "sticht"], make_sololike(ESoloLike::Solo)),
        (&["wenz"], make_sololike(ESoloLike::Wenz)),
        (&["geier"], make_sololike(ESoloLike::Geier)),
        (&["bettel normal"], {
            get_epi_active().map(|epi| {
                Box::new(SRulesBettel::<SBettelAllAllowedCardsWithinStichNormal>::new(
                    epi,
                    /*i_prio*/-999_999,
                    /*n_payout_base*/n_tarif_solo,
                )) as Box<dyn TRules>
            })
        }),
        (&["bettel stich"], {
            get_epi_active().map(|epi| {
                Box::new(SRulesBettel::<SBettelAllAllowedCardsWithinStichStichzwang>::new(
                    epi,
                    /*i_prio*/-999_999,
                    /*n_payout_base*/n_tarif_solo,
                )) as Box<dyn TRules>
            })
        }),
        (&["ramsch"], {
            Ok(Box::new(SRulesRamsch::new(
                /*n_price*/n_tarif_ruf,
                VDurchmarsch::AtLeast(91), // https://www.sauspiel.de/blog/66-bei-sauspiel-wird-jetzt-mit-ramsch-gespielt
                Some(VJungfrau::DoubleAll),
            )) as Box<dyn TRules>)
        }),
    ].into_iter()
        .filter(|(slcstr, _)| str_rules_contains(slcstr))
        .exactly_one()
    {
        Ok((_, Ok(rules))) => Ok(rules),
        Ok((_, Err(ref e))) => Err(format_err!("{}", &e.to_string())),
        Err(itslcstrrules) => {
            if 0==itslcstrrules.count() {
                Err(format_err!("Cannot understand rule description."))
            } else {
                Err(format_err!("Rule description is ambiguous."))
            }
        }
    }
}

pub fn parse_rule_description_simple(str_rules: &str) -> Result<Box<dyn TRules>, Error> {
    crate::rules::parser::parse_rule_description(
        str_rules,
        (/*n_tarif_extra*/10, /*n_tarif_ruf*/20, /*n_tarif_solo*/50), // TODO? make customizable
        /*fn_player_to_epi*/|str_epi| EPlayerIndex::checked_from_usize(str_epi.parse()?)
            .ok_or_else(|| format_err!("Cannot convert {} to EPlayerIndex.", str_epi)),
    )
}
