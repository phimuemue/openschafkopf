use crate::ai::{SDetermineBestCardResult, SPayoutStats, SPayoutStatsPerStrategy, gametree::EMinMaxStrategy};
use crate::primitives::*;
use itertools::*;
use crate::util::*;
use crate::rules::TRules;
use std::borrow::Borrow;

pub const N_COLUMNS : usize = 4;

// crude formatting: treat all numbers as f32, and convert structured input to a plain number table
#[derive(PartialEq)]
pub struct SOutputLine {
    pub veccard: Vec<SCard>,
    pub mapemmstrategyatplstrf: EnumMap<EMinMaxStrategy, [(String, f32); N_COLUMNS]>,
}

#[derive(Clone, /*TODO really needed for array construction?*/Copy)]
pub struct SFormatInfo {
    pub f_min: f32,
    pub f_max: f32,
    pub n_width: usize,
}

pub fn internal_table<PayoutStatsPerStrategy: Borrow<SPayoutStatsPerStrategy>>(
    mut veccardpayoutstatsperstrategy: Vec<(SCard, PayoutStatsPerStrategy)>,
    fn_human_readable_payout: &dyn Fn(f32) -> f32,
) -> (Vec<SOutputLine>, EnumMap<EMinMaxStrategy, [SFormatInfo; N_COLUMNS]>) {
    veccardpayoutstatsperstrategy.sort_unstable_by(|(_card_lhs, minmax_lhs), (_card_rhs, minmax_rhs)| {
        minmax_lhs.borrow().compare_canonical(minmax_rhs.borrow())
    });
    veccardpayoutstatsperstrategy.reverse(); // descending
    let mut vecoutputline : Vec<SOutputLine> = Vec::new();
    let mut mapemmstrategyaformatinfo = EMinMaxStrategy::map_from_fn(|_emmstrategy| [
        SFormatInfo {
            f_min: f32::MAX,
            f_max: f32::MIN,
            n_width: 0,
        };
        N_COLUMNS
    ]);
    for (mapemmstrategyatplstrf, grptplcardmapemmstrategyatplstrf) in veccardpayoutstatsperstrategy.into_iter()
        .map(|(card, minmax)| {
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
                card,
                EMinMaxStrategy::map_from_fn(|emmstrategy| [
                    column_min_or_max(minmax.0[emmstrategy].min()),
                    column_average(&minmax.0[emmstrategy]),
                    column_min_or_max(minmax.0[emmstrategy].max()),
                    column_counts(&minmax.0[emmstrategy]),
                ]),
            )
        })
        .group_by(|(_card, mapemmstrategyatplstrf)| mapemmstrategyatplstrf.clone())
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
            veccard: grptplcardmapemmstrategyatplstrf.into_iter()
                .map(|(card, _atplstrf)| card)
                .collect(),
            mapemmstrategyatplstrf,
        });
    }
    (vecoutputline, mapemmstrategyaformatinfo)
}

pub fn table(
    determinebestcardresult: &SDetermineBestCardResult<SPayoutStatsPerStrategy>,
    rules: &dyn TRules,
    fn_human_readable_payout: &dyn Fn(f32) -> f32,
) -> (Vec<SOutputLine>, usize/*n_max_cards*/, EnumMap<EMinMaxStrategy, [SFormatInfo; N_COLUMNS]>) {
    let mut n_max_cards = 0;
    let (mut vecoutputline, mapemmstrategyaformatinfo) = internal_table(
        determinebestcardresult.cards_and_ts().collect(),
        fn_human_readable_payout,
    );
    for outputline in vecoutputline.iter_mut() {
        rules.sort_cards_first_trumpf_then_farbe(&mut outputline.veccard);
        assign_max(&mut n_max_cards, outputline.veccard.len());
    }
    (vecoutputline, n_max_cards, mapemmstrategyaformatinfo)
}
