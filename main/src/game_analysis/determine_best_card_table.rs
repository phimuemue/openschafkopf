use crate::ai::{SDetermineBestCardResult, SPayoutStats, SPayoutStatsPerStrategy, gametree::EMinMaxStrategy};
use crate::primitives::*;
use itertools::*;
use crate::util::*;
use crate::rules::TRules;
use std::borrow::Borrow;

pub const N_COLUMNS : usize = 4;

// crude formatting: treat all numbers as f32, and convert structured input to a plain number table
#[derive(PartialEq)]
pub struct SOutputLine<T> {
    pub vect: Vec<T>,
    pub mapemmstrategyatplstrf: EnumMap<EMinMaxStrategy, [(String, f32); N_COLUMNS]>,
}

#[derive(Clone, /*TODO really needed for array construction?*/Copy)]
pub struct SFormatInfo {
    pub f_min: f32,
    pub f_max: f32,
    pub n_width: usize,
}

enum EGrouping { Group, NoGroup }
impl PartialEq for EGrouping {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (EGrouping::Group, EGrouping::Group))
    }
}

pub struct SPayoutStatsTable<T> {
    pub vecoutputline: Vec<SOutputLine<T>>,
    mapemmstrategyaformatinfo: EnumMap<EMinMaxStrategy, [SFormatInfo; N_COLUMNS]>,
}

pub fn internal_table<T, PayoutStatsPerStrategy: Borrow<SPayoutStatsPerStrategy>>(
    mut vectpayoutstatsperstrategy: Vec<(T, PayoutStatsPerStrategy)>,
    b_group: bool,
    fn_human_readable_payout: &dyn Fn(f32) -> f32,
) -> SPayoutStatsTable<T> {
    vectpayoutstatsperstrategy.sort_unstable_by(|(_t_lhs, minmax_lhs), (_t_rhs, minmax_rhs)| {
        minmax_lhs.borrow().compare_canonical(minmax_rhs.borrow())
    });
    vectpayoutstatsperstrategy.reverse(); // descending
    let mut vecoutputline : Vec<SOutputLine<_>> = Vec::new();
    let mut mapemmstrategyaformatinfo = EMinMaxStrategy::map_from_fn(|_emmstrategy| [
        SFormatInfo {
            f_min: f32::MAX,
            f_max: f32::MIN,
            n_width: 0,
        };
        N_COLUMNS
    ]);
    for ((mapemmstrategyatplstrf, _grouping), grptpltmapemmstrategyatplstrf) in vectpayoutstatsperstrategy.into_iter()
        .map(|(t, minmax)| {
            let minmax = minmax.borrow();
            let column_counts = |paystats: &SPayoutStats| {(
                format!("{} ", paystats.counts().iter().join("/")),
                (paystats.counts()[std::cmp::Ordering::Equal]+paystats.counts()[std::cmp::Ordering::Greater])
                    .as_num::<f32>(),
            )};
            let column_min_or_max = |n: isize| {
                let f_human_readable_payout = fn_human_readable_payout(n.as_num::<f32>());
                (format!("{} ", f_human_readable_payout), f_human_readable_payout)
            };
            let column_average = |paystats: &SPayoutStats| {
                let f_human_readable_payout = fn_human_readable_payout(paystats.avg());
                (format!("{:.2} ", f_human_readable_payout), f_human_readable_payout)
            };
            (
                t,
                EMinMaxStrategy::map_from_fn(|emmstrategy| [
                    column_min_or_max(minmax.0[emmstrategy].min()),
                    column_average(&minmax.0[emmstrategy]),
                    column_min_or_max(minmax.0[emmstrategy].max()),
                    column_counts(&minmax.0[emmstrategy]),
                ]),
            )
        })
        .group_by(|(_t, mapemmstrategyatplstrf)| {
            (
                mapemmstrategyatplstrf.clone(),
                if b_group {
                    EGrouping::Group
                } else {
                    EGrouping::NoGroup
                },
            )
        })
        .into_iter()
    {
        for (atplstrf, aformatinfo) in mapemmstrategyatplstrf.iter().zip_eq(mapemmstrategyaformatinfo.iter_mut()) {
            for ((str_val, f_val), formatinfo) in atplstrf.iter().zip_eq(aformatinfo.iter_mut()) {
                formatinfo.n_width = formatinfo.n_width.max(str_val.len());
                assign_min_partial_ord(&mut formatinfo.f_min, *f_val);
                assign_max_partial_ord(&mut formatinfo.f_max, *f_val);
            }
        }
        vecoutputline.push(SOutputLine{
            vect: grptpltmapemmstrategyatplstrf.into_iter()
                .map(|(t, _atplstrf)| t)
                .collect(),
            mapemmstrategyatplstrf,
        });
    }
    SPayoutStatsTable{
        vecoutputline,
        mapemmstrategyaformatinfo,
    }
}

pub fn table(
    determinebestcardresult: &SDetermineBestCardResult<SPayoutStatsPerStrategy>,
    rules: &dyn TRules,
    fn_human_readable_payout: &dyn Fn(f32) -> f32,
) -> SPayoutStatsTable<SCard> {
    let mut payoutstatstable = internal_table(
        determinebestcardresult.cards_and_ts().collect(),
        /*b_group*/true,
        fn_human_readable_payout,
    );
    for outputline in payoutstatstable.vecoutputline.iter_mut() {
        rules.sort_cards_first_trumpf_then_farbe(&mut outputline.vect);
    }
    payoutstatstable
}

impl<T> SPayoutStatsTable<T> {
    pub fn print(
        &self,
        b_verbose: bool,
    ) where T: std::fmt::Display {
        let slcoutputline = &self.vecoutputline;
        if b_verbose { // TODO? only for second-level verbosity
            println!("\nInterpreting a line of the following table (taking the first line as an example):");
            let SOutputLine{vect, mapemmstrategyatplstrf} = &slcoutputline[0];
            println!("If you play {}, then:", vect.iter().join(" or "));
            for emmstrategy in EMinMaxStrategy::values() {
                let astr = mapemmstrategyatplstrf[emmstrategy].clone().map(|tplstrf| tplstrf.0);
                let n_columns = astr.len(); // TODO can we get rid of this
                let [str_payout_min, str_payout_avg, str_payout_max, str_stats] = astr;
                println!("* The {} {} columns show tell what happens if all other players play {}:",
                    EMinMaxStrategy::map_from_raw([
                        "first",
                        "second",
                        "third",
                        "fourth",
                        "fifth",
                    ])[emmstrategy],
                    n_columns,
                    match emmstrategy {
                        EMinMaxStrategy::MinMin => "adversarially and you play pessimal",
                        EMinMaxStrategy::Min => "adversarially",
                        EMinMaxStrategy::SelfishMin => "optimally for themselves, favouring you in case of doubt",
                        EMinMaxStrategy::SelfishMax => "optimally for themselves, not favouring you in case of doubt",
                        EMinMaxStrategy::Max => "optimally for you",
                    },
                );
                println!("  * In the worst case (over all generated card distributions), you can enforce a payout of {}", str_payout_min);
                println!("  * On average (over all generated card distributions), you can enforce a payout of {}", str_payout_avg);
                println!("  * In the best case (over all generated card distributions), you can enforce a payout of {}", str_payout_max);
                println!("  * {} shows the number of games lost/zero-payout/won", str_stats);
            }
            println!();
        }
        // TODO interface should probably output payout interval per card
        let mut vecstr_id = Vec::new();
        let mut n_width_id = 0;
        for outputline in slcoutputline.iter() {
            let str_id = outputline.vect.iter().join(" ");
            assign_max(&mut n_width_id, str_id.len());
            vecstr_id.push(str_id);
        }
        for (str_id, SOutputLine{vect:_, mapemmstrategyatplstrf}) in vecstr_id.iter().zip_eq(slcoutputline.iter()) {
            print!("{str_id:<n_width_id$}: ");
            for (atplstrf, aformatinfo) in mapemmstrategyatplstrf.iter().zip_eq(self.mapemmstrategyaformatinfo.iter()) {
                for ((str_num, f), SFormatInfo{f_min, f_max, n_width}) in atplstrf.iter().zip_eq(aformatinfo.iter()) {
                    use termcolor::*;
                    let mut stdout = StandardStream::stdout(if atty::is(atty::Stream::Stdout) {
                        ColorChoice::Auto
                    } else {
                        ColorChoice::Never
                    });
                    #[allow(clippy::float_cmp)]
                    if f_min!=f_max {
                        let mut set_color = |color| {
                            unwrap!(stdout.set_color(ColorSpec::new().set_fg(Some(color))));
                        };
                        if f==f_min {
                            set_color(Color::Red);
                        } else if f==f_max {
                            set_color(Color::Green);
                        }
                    }
                    print!("{:>width$}", str_num, width=n_width);
                    unwrap!(stdout.reset());
                }
                print!("   ");
            }
            println!();
        }
    }
}
