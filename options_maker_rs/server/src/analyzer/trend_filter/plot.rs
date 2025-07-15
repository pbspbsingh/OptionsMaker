use crate::analyzer::dataframe::DataFrame;
use plotly::common::Mode;
use plotly::layout::{Axis, LayoutGrid};
use plotly::{Candlestick, Layout, Plot, Scatter};
use std::time::Duration;

pub fn plot_df(df: &DataFrame, dur: &Duration) {
    let candlestick = Candlestick::new(
        df.index().to_vec(),
        df["open"].to_vec(),
        df["high"].to_vec(),
        df["low"].to_vec(),
        df["close"].to_vec(),
    )
    .name("Price")
    .x_axis("x")
    .y_axis("y");

    let volume_trace = Scatter::new(df.index().to_vec(), df["volume"].to_vec())
        .name("Volume")
        .mode(Mode::Lines)
        .x_axis("x")
        .y_axis("y2");

    let layout = Layout::new()
        .grid(
            LayoutGrid::new()
                .rows(2)
                .columns(1)
                .pattern(plotly::layout::GridPattern::Independent),
        )
        .x_axis(Axis::new().domain(&[0.0, 1.0]).anchor("y"))
        .y_axis(Axis::new().domain(&[0.3, 1.0]).anchor("x"))
        .y_axis2(Axis::new().domain(&[0.0, 0.25]).anchor("x"))
        .height(800);

    let mut plot = Plot::new();
    plot.add_trace(Box::new(candlestick));
    plot.add_trace(volume_trace);
    plot.set_layout(layout);

    std::fs::write(
        format!("plots/{}-mins.html", dur.as_secs() / 60),
        plot.to_html(),
    )
    .unwrap();
}
