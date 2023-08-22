use crate::ai::{SDetermineBestCardResult, SPayoutStats, SPerMinMaxStrategy, gametree::EMinMaxStrategy};
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
    vecoutputline: Vec<SOutputLine<T>>,
    mapemmstrategyaformatinfo: EnumMap<EMinMaxStrategy, [SFormatInfo; N_COLUMNS]>,
}
impl<T> SPayoutStatsTable<T> {
    // TODO? would an accessor macro be helpful?
    pub fn output_lines(&self) -> &Vec<SOutputLine<T>> {
        &self.vecoutputline
    }
    pub fn into_output_lines(self) -> Vec<SOutputLine<T>> {
        self.vecoutputline
    }
    pub fn format_infos(&self) -> &EnumMap<EMinMaxStrategy, [SFormatInfo; N_COLUMNS]> {
        &self.mapemmstrategyaformatinfo
    }
}

pub fn internal_table<T, PayoutStatsPayload: Copy+Ord+std::fmt::Debug, PayoutStatsPerStrategy: Borrow<SPerMinMaxStrategy<SPayoutStats<PayoutStatsPayload>>>>(
    mut vectpayoutstatsperstrategy: Vec<(T, PayoutStatsPerStrategy)>,
    b_group: bool,
    fn_loss_or_win: &dyn Fn(isize, PayoutStatsPayload) -> std::cmp::Ordering,
) -> SPayoutStatsTable<T> {
    vectpayoutstatsperstrategy.sort_unstable_by(|(_t_lhs, minmax_lhs), (_t_rhs, minmax_rhs)| {
        minmax_lhs.borrow().compare_canonical(minmax_rhs.borrow(), fn_loss_or_win)
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
            let column_counts = |paystats: &SPayoutStats<PayoutStatsPayload>| {
                let mapordn_count = paystats.counts(fn_loss_or_win);
                (
                    format!("{} ", mapordn_count.iter().join("/")),
                    (mapordn_count[std::cmp::Ordering::Equal]+mapordn_count[std::cmp::Ordering::Greater])
                        .as_num::<f32>(),
                )
            };
            let column_min_or_max = |n: isize| {
                (format!("{} ", n), n.as_num::<f32>())
            };
            let column_average = |paystats: &SPayoutStats<PayoutStatsPayload>| {
                let f_avg = paystats.avg();
                (format!("{:.2} ", f_avg), f_avg)
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

pub fn table<PayoutStatsPayload: Copy+Ord+std::fmt::Debug>(
    determinebestcardresult: &SDetermineBestCardResult<SPerMinMaxStrategy<SPayoutStats<PayoutStatsPayload>>>,
    rules: &dyn TRules,
    fn_loss_or_win: &dyn Fn(isize, PayoutStatsPayload) -> std::cmp::Ordering,
) -> SPayoutStatsTable<ECard> {
    let mut payoutstatstable = internal_table(
        determinebestcardresult.cards_and_ts().collect(),
        /*b_group*/true,
        fn_loss_or_win,
    );
    for outputline in payoutstatstable.vecoutputline.iter_mut() {
        rules.sort_cards_first_trumpf_then_farbe(&mut outputline.vect);
    }
    payoutstatstable
}
