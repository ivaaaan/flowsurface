use crate::chart::{
    indicator::{
        indicator_row, kline::{FetchCtx, KlineIndicatorImpl}, plot::{line::LinePlot, PlotTooltip}
    }, Caches, Message, ViewState
};

use data::{chart::{kline::KlineDataPoint, Basis, PlotData}, util::format_with_commas};
use exchange::{adapter::Exchange, fetcher::FetchRange, Kline, Timeframe, Trade};
use iced::widget::{center, row, text};
use std::{collections::BTreeMap, ops::RangeInclusive};


pub struct FundingRateIndicator {
    pub data: BTreeMap<u64, f32>,
    pub cache: Caches,
}

impl FundingRateIndicator {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            cache: Caches::default(),
        }
    }

    fn indicator_elem<'a>(
        &'a self,
        main_chart: &'a ViewState,
        visible_range: RangeInclusive<u64>,
    ) -> iced::Element<'a, Message> {
        match main_chart.basis {
            Basis::Time(timeframe) => {
                let exchange = main_chart.ticker_info.exchange();
                if !Self::is_supported_exchange(exchange) {
                    return center(text(format!(
                        "WIP: Open Interest is not available for {exchange}"
                    )))
                    .into();
                }

                let (earliest, latest) = visible_range.clone().into_inner();
                if latest < earliest {
                    return row![].into();
                }
            }
            Basis::Tick(_) => {
                return center(text("WIP: Open Interest is not available for tick charts.")).into();
            }
        }

        let tooltip = |value: &f32, next: Option<&f32>| {
            let value_text = format!("Open Interest: {}", format_with_commas(*value));
            let change_text = if let Some(next_value) = next {
                let delta = next_value - *value;
                let sign = if delta >= 0.0 { "+" } else { "" };
                format!("Change: {}{}", sign, format_with_commas(delta))
            } else {
                "Change: N/A".to_string()
            };
            PlotTooltip::new(format!("{value_text}\n{change_text}"))
        };

        let value_fn = |v: &f32| *v;

        let plot = LinePlot::new(value_fn)
            .stroke_width(1.0)
            .show_points(true)
            .point_radius_factor(0.2)
            .padding(0.08)
            .with_tooltip(tooltip);

        indicator_row(main_chart, &self.cache, plot, &self.data, visible_range)
    }

    fn fr_timerange(&self, latest_kline: u64) -> (u64, u64) {
        let mut from_time = latest_kline;
        let mut to_time = u64::MIN;

        self.data.iter().for_each(|(time, _)| {
            from_time = from_time.min(*time);
            to_time = to_time.max(*time);
        });
        (from_time, to_time)
    }


    pub fn is_supported_exchange(exchange: Exchange) -> bool {
        exchange.is_perps() && exchange != Exchange::HyperliquidLinear
    }
}

impl KlineIndicatorImpl for FundingRateIndicator {
    fn clear_all_caches(&mut self) {
        self.cache.clear_all();
    }

    fn clear_crosshair_caches(&mut self) {
        self.cache.clear_crosshair();
    }

    fn element<'a>(
        &'a self,
        chart: &'a ViewState,
        visible_range: RangeInclusive<u64>,
    ) -> iced::Element<'a, Message> {
        self.indicator_elem(chart, visible_range)
    }

    fn fetch_range(&mut self, ctx: &FetchCtx) -> Option<FetchRange> {
        let exchange = ctx.main_chart.ticker_info.exchange();
        let is_supported = Self::is_supported_exchange(exchange);

        if !is_supported {
            return None;
        }

        let (fr_earliest, fr_latest) = self.fr_timerange(ctx.kline_latest);

        if ctx.visible_earliest < fr_earliest {
            return Some(FetchRange::FundingRate(ctx.prefetch_earliest, fr_earliest));
        }

        if fr_latest < ctx.kline_latest {
            return Some(FetchRange::FundingRate(
                fr_latest.max(ctx.prefetch_earliest),
                ctx.kline_latest,
            ));
        }

        None

   }

    fn on_insert_klines(&mut self, _klines: &[Kline]) {
        // Handle new klines if necessary
    }

    fn on_insert_trades(
        &mut self,
        _trades: &[Trade],
        _old_dp_len: usize,
        _source: &PlotData<KlineDataPoint>,
    ) {}

    fn on_ticksize_change(&mut self, _source: &PlotData<KlineDataPoint>) {}

    fn on_basis_change(&mut self, _source: &PlotData<KlineDataPoint>) {}

    fn on_funding_rate(&mut self, data: &[exchange::FundingRate]) {
        self.data.extend(data.iter().map(|fr| (fr.time, fr.rate)));
        self.clear_all_caches();
    }
}
