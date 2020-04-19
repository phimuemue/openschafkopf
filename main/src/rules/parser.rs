use crate::util::*;
use crate::primitives::*;
use crate::game_analysis::*;
use crate::rules::*;
use itertools::Itertools;

pub fn parse_rule_description(
    str_rules_with_player: &str,
    (n_tarif_extra, n_tarif_ruf, n_tarif_solo): (isize, isize, isize),
    fn_player_to_epi: impl FnOnce(&str)->Result<EPlayerIndex, Error>,
) -> Result<Box<dyn TRules>, Error> {
    use crate::rules::rulesrufspiel::*;
    use crate::rules::rulessolo::*;
    use crate::rules::rulesramsch::*;
    use crate::rules::payoutdecider::*;
    let vecstr_rule_parts = str_rules_with_player.split(" von ").collect::<Vec<_>>();
    let epi_active = if_then_some!(2==vecstr_rule_parts.len(), vecstr_rule_parts[1])
        .ok_or_else(|| format_err!("Cannot determine active player: {}", str_rules_with_player)) // TODO not needed for ramsch
        .and_then(fn_player_to_epi)?;
    // Regarding laufende:
    // https://www.sauspiel.de/hilfe#71-beim-farbwenz-wurden-meine-laufende-nicht-berechnet
    // https://www.schafkopfschule.de/index.php/regeln.html?file=files/inhalte/dokumente/Spielen/Regeln/Schafkopfregeln-Aktuell-29.3.2007.pdf (Section 4.2 Spielabrechnung)
    let str_rules = vecstr_rule_parts[0].to_lowercase();
    let str_rules_contains = |slcstr: &[&str]| slcstr.iter().any(|str| str_rules.contains(str));
    // determine oefarbe
    let oefarbe = match [
        (EFarbe::Eichel, &["eichel", "alt"] as &[&str]),
        (EFarbe::Gras, &["gras", "grün", "laub", "blatt", "blau"]),
        (EFarbe::Herz, &["herz", "rot"]),
        (EFarbe::Schelln, &["schelln", "schelle", "pump", "hundsgfickte"]),
    ].iter()
        .filter(|(_efarbe, slcstr_farbe)| str_rules_contains(slcstr_farbe))
        .exactly_one()
    {
        Ok((efarbe, _)) => Some(*efarbe),
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
                epi_active,
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
        }};
        if str_rules_contains(&["tout"]) {
            make_sololike_internal!(SPayoutDeciderTout)
        } else {
            make_sololike_internal!(SPayoutDeciderPointBased)
        }
    };
    match [
        (&["rufspiel", "sauspiel"] as &[&str], {
            match oefarbe {
                None => Err(format_err!("Rufspiel requires efarbe")),
                Some(efarbe) => match efarbe {
                    EFarbe::Herz => Err(format_err!("Rufspiel incompatible with EFarbe::Herz")),
                    EFarbe::Eichel | EFarbe::Gras | EFarbe::Schelln => {
                        Ok(Box::new(SRulesRufspiel::new(
                            epi_active,
                            efarbe,
                            SPayoutDeciderParams::new(
                                /*n_payout_base*/n_tarif_ruf,
                                /*n_payout_schneider_schwarz*/n_tarif_extra,
                                SLaufendeParams::new(
                                    /*n_payout_single_player*/n_tarif_extra,
                                    /*n_lauf_lbound*/3,
                                ),
                            )
                        )) as Box<dyn TRules>)
                    }
                }
            }
        }),
        (&["solo", "sticht"], make_sololike(ESoloLike::Solo)),
        (&["wenz"], make_sololike(ESoloLike::Wenz)),
        (&["geier"], make_sololike(ESoloLike::Geier)),
        (&["ramsch"], {
            Ok(Box::new(SRulesRamsch::new(
                /*n_price*/n_tarif_ruf,
                VDurchmarsch::AtLeast(91), // https://www.sauspiel.de/blog/66-bei-sauspiel-wird-jetzt-mit-ramsch-gespielt
                // TODO Jungfrau
            )) as Box<dyn TRules>)
        }),
    ].iter()
        .filter(|(slcstr, _)| str_rules_contains(slcstr))
        .exactly_one()
    {
        Ok((_, Ok(rules))) => Ok(rules.box_clone()), // TODORUST avoid box_clone once arrays support proper into_iter
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


