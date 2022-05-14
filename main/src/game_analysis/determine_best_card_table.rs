use crate::ai::{SDetermineBestCardResult, SPayoutStats, SPayoutStatsPerStrategy};
use crate::primitives::*;
use itertools::*;
use crate::util::*;
use crate::rules::TRules;

pub const N_COLUMNS : usize = 16;

// crude formatting: treat all numbers as f32, and convert structured input to a plain number table
#[derive(PartialEq)]
pub struct SOutputLine {
    pub veccard: Vec<SCard>,
    pub atplstrf: [(String, f32); N_COLUMNS],
}

#[derive(Clone, /*TODO really needed for array construction?*/Copy)]
pub struct SFormatInfo {
    pub f_min: f32,
    pub f_max: f32,
    pub n_width: usize,
}

pub fn table(
    determinebestcardresult: &SDetermineBestCardResult<SPayoutStatsPerStrategy>,
    rules: &dyn TRules,
    fn_human_readable_payout: &dyn Fn(f32) -> f32,
) -> (Vec<SOutputLine>, usize/*n_max_cards*/, [SFormatInfo; N_COLUMNS]) {
    let mut n_max_cards = 0;
    let mut veccardminmax = determinebestcardresult.cards_and_ts().collect::<Vec<_>>();
    veccardminmax.sort_unstable_by(|&(_card_lhs, minmax_lhs), &(_card_rhs, minmax_rhs)| {
        minmax_lhs.compare_canonical(minmax_rhs)
    });
    veccardminmax.reverse(); // descending
    let mut vecoutputline : Vec<SOutputLine> = Vec::new();
    let mut aformatinfo = [
        SFormatInfo {
            f_min: f32::MAX,
            f_max: f32::MIN,
            n_width: 0,
        };
        N_COLUMNS
    ];
    for (atplstrf, grptplcardatplstrf) in veccardminmax.into_iter()
        .map(|(card, minmax)| {
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
                [
                    column_min_or_max(minmax.t_min.min()),
                    column_average(&minmax.t_min),
                    column_min_or_max(minmax.t_min.max()),
                    column_counts(&minmax.t_min),
                    column_min_or_max(minmax.t_selfish_min.min()),
                    column_average(&minmax.t_selfish_min),
                    column_min_or_max(minmax.t_selfish_min.max()),
                    column_counts(&minmax.t_selfish_min),
                    column_min_or_max(minmax.t_selfish_max.min()),
                    column_average(&minmax.t_selfish_max),
                    column_min_or_max(minmax.t_selfish_max.max()),
                    column_counts(&minmax.t_selfish_max),
                    column_min_or_max(minmax.t_max.min()),
                    column_average(&minmax.t_max),
                    column_min_or_max(minmax.t_max.max()),
                    column_counts(&minmax.t_max),
                ],
            )
        })
        .group_by(|(_card, atplstrf)| atplstrf.clone())
        .into_iter()
    {
        for ((str_val, f_val), formatinfo) in atplstrf.iter().zip_eq(aformatinfo.iter_mut()) {
            formatinfo.n_width = formatinfo.n_width.max(str_val.len());
            assign_min_partial_ord(&mut formatinfo.f_min, *f_val);
            assign_max_partial_ord(&mut formatinfo.f_max, *f_val);
        }
        let mut veccard : Vec<_> = grptplcardatplstrf.into_iter()
            .map(|(card, _atplstrf)| card)
            .collect();
        rules.sort_cards_first_trumpf_then_farbe(&mut veccard);
        assign_max(&mut n_max_cards, veccard.len());
        vecoutputline.push(SOutputLine{
            veccard,
            atplstrf,
        });
    }
    (vecoutputline, n_max_cards, aformatinfo)
}
