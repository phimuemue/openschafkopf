use crate::ai::{
    gametree::{TMinMaxStrategiesHigherKinded, TMinMaxStrategies},
    SDetermineBestCardResult, SPayoutStats,
};
use crate::primitives::*;
use itertools::*;
use crate::util::*;
use crate::rules::SRules;
use std::borrow::Borrow;

pub const N_COLUMNS : usize = 4;

// crude formatting: treat all numbers as f32, and convert structured input to a plain number table
#[derive(PartialEq)]
pub struct SOutputLine<T, MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded> {
    pub vect: Vec<T>,
    pub perminmaxstrategyatplstrf: MinMaxStrategiesHK::Type<[(String, f32); N_COLUMNS]>,
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

pub struct SPayoutStatsTable<T, MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded> {
    vecoutputline: Vec<SOutputLine<T, MinMaxStrategiesHK>>,
    perminmaxstrategyaformatinfo: MinMaxStrategiesHK::Type<[SFormatInfo; N_COLUMNS]>,
}
impl<T, MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded> SPayoutStatsTable<T, MinMaxStrategiesHK> {
    // TODO? would an accessor macro be helpful?
    pub fn output_lines(&self) -> &Vec<SOutputLine<T, MinMaxStrategiesHK>> {
        &self.vecoutputline
    }
    pub fn into_output_lines(self) -> Vec<SOutputLine<T, MinMaxStrategiesHK>> {
        self.vecoutputline
    }
    pub fn format_infos(&self) -> &MinMaxStrategiesHK::Type<[SFormatInfo; N_COLUMNS]> {
        &self.perminmaxstrategyaformatinfo
    }
}

pub fn internal_table<
    MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded,
    T,
    PayoutStatsPayload: Copy+Ord+std::fmt::Debug,
    PayoutStatsPerStrategy: Borrow<MinMaxStrategiesHK::Type<SPayoutStats<PayoutStatsPayload>>>,
>(
    mut vectpayoutstatsperstrategy: Vec<(T, PayoutStatsPerStrategy)>,
    b_group: bool,
    fn_loss_or_win: &dyn Fn(isize, PayoutStatsPayload) -> std::cmp::Ordering,
) -> SPayoutStatsTable<T, MinMaxStrategiesHK>
    where
        MinMaxStrategiesHK::Type<[(String, f32); N_COLUMNS]>: PartialEq+Clone,
{
    vectpayoutstatsperstrategy.sort_unstable_by(|(_t_lhs, minmax_lhs), (_t_rhs, minmax_rhs)| {
        minmax_lhs.borrow().compare_canonical(minmax_rhs.borrow(), fn_loss_or_win)
    });
    vectpayoutstatsperstrategy.reverse(); // descending
    let mut vecoutputline : Vec<SOutputLine<_,_>> = Vec::new();
    let mut perminmaxstrategyaformatinfo = MinMaxStrategiesHK::Type::new([
        SFormatInfo {
            f_min: f32::MAX,
            f_max: f32::MIN,
            n_width: 0,
        };
        N_COLUMNS
    ]);
    for ((perminmaxstrategyatplstrf, _grouping), grptpltmapemmstrategyatplstrf) in vectpayoutstatsperstrategy.into_iter()
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
                minmax.map(|payoutstats| [
                    column_min_or_max(payoutstats.min()),
                    column_average(&payoutstats),
                    column_min_or_max(payoutstats.max()),
                    column_counts(&payoutstats),
                ]),
            )
        })
        .group_by(|(_t, perminmaxstrategyatplstrf)| {
            (
                perminmaxstrategyatplstrf.clone(),
                if b_group {
                    EGrouping::Group
                } else {
                    EGrouping::NoGroup
                },
            )
        })
        .into_iter()
    {
        perminmaxstrategyaformatinfo.modify_with_other(&perminmaxstrategyatplstrf, |aformatinfo, atplstrf| {
            for ((str_val, f_val), formatinfo) in atplstrf.iter().zip_eq(aformatinfo.iter_mut()) {
                formatinfo.n_width = formatinfo.n_width.max(str_val.len());
                assign_min_partial_ord(&mut formatinfo.f_min, *f_val);
                assign_max_partial_ord(&mut formatinfo.f_max, *f_val);
            }
        });
        vecoutputline.push(SOutputLine{
            vect: grptpltmapemmstrategyatplstrf.into_iter()
                .map(|(t, _atplstrf)| t)
                .collect(),
            perminmaxstrategyatplstrf,
        });
    }
    SPayoutStatsTable{
        vecoutputline,
        perminmaxstrategyaformatinfo,
    }
}

pub fn table<
    MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded,
    PayoutStatsPayload: Copy+Ord+std::fmt::Debug
>(
    determinebestcardresult: &SDetermineBestCardResult<MinMaxStrategiesHK::Type<SPayoutStats<PayoutStatsPayload>>>,
    rules: &SRules,
    fn_loss_or_win: &dyn Fn(isize, PayoutStatsPayload) -> std::cmp::Ordering,
) -> SPayoutStatsTable<ECard, MinMaxStrategiesHK>
    where
        MinMaxStrategiesHK::Type<SPayoutStats<PayoutStatsPayload>>: std::fmt::Debug, // TODO needed?
        MinMaxStrategiesHK::Type<[(String, f32); N_COLUMNS]>: PartialEq+Clone,
{
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
