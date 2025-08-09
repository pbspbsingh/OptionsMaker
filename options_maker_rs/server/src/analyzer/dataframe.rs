use chrono::{Datelike, NaiveDate, NaiveDateTime, Weekday};
use itertools::Itertools;
use schwab_client::Candle;
use serde_json::{Map, Value};
use std::fmt::{Debug, Display, Write};

use rustc_hash::FxHashMap;
use std::ops::Index;

#[derive(Clone)]
pub struct DataFrame {
    index: Vec<NaiveDateTime>,
    col_names: Vec<String>,
    columns: FxHashMap<String, Vec<f64>>,
}

impl DataFrame {
    pub fn from_cols(cols: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let index = Vec::new();
        let col_names = cols.into_iter().map(|s| s.into()).collect::<Vec<_>>();
        let columns = col_names
            .iter()
            .map(|name| (name.clone(), Vec::new()))
            .collect();
        DataFrame {
            index,
            col_names,
            columns,
        }
    }

    pub fn from_candles(candles: &[Candle]) -> Self {
        let mut index = Vec::with_capacity(candles.len());
        let mut opens = Vec::with_capacity(candles.len());
        let mut lows = Vec::with_capacity(candles.len());
        let mut highs = Vec::with_capacity(candles.len());
        let mut closes = Vec::with_capacity(candles.len());
        let mut volumes = Vec::with_capacity(candles.len());
        for candle in candles {
            index.push(candle.time.naive_local());
            opens.push(candle.open);
            lows.push(candle.low);
            highs.push(candle.high);
            closes.push(candle.close);
            volumes.push(candle.volume as f64);
        }
        let mut df = Self {
            index,
            col_names: Vec::new(),
            columns: FxHashMap::default(),
        };
        df.insert_column("open", opens).unwrap();
        df.insert_column("low", lows).unwrap();
        df.insert_column("high", highs).unwrap();
        df.insert_column("close", closes).unwrap();
        df.insert_column("volume", volumes).unwrap();
        df
    }

    pub fn shape(&self) -> (usize, usize) {
        (self.index.len(), self.col_names.len())
    }

    pub fn insert_column(&mut self, name: impl Into<String>, data: Vec<f64>) -> anyhow::Result<()> {
        if data.len() != self.index.len() {
            return Err(anyhow::anyhow!(
                "Length of column to be inserted ({}) should be same as index length ({})",
                data.len(),
                self.index.len()
            ));
        }

        let col_name = name.into();
        if !self.columns.contains_key(&col_name) {
            self.col_names.push(col_name.clone());
        }
        self.columns.insert(col_name, data);
        Ok(())
    }

    pub fn index(&self) -> &[NaiveDateTime] {
        &self.index
    }

    pub fn column_names(&self) -> Vec<String> {
        ["index".to_owned()]
            .into_iter()
            .chain(self.col_names.iter().cloned())
            .collect()
    }

    pub fn trim_working_days(&self, days: usize) -> Self {
        let min_working_hours = util::time::regular_trading_hours();
        let work_days = self
            .index
            .iter()
            .fold(
                FxHashMap::<NaiveDate, (NaiveDateTime, NaiveDateTime)>::default(),
                |mut map, &idx| {
                    let entry = map.entry(idx.date()).or_insert_with(|| (idx, idx));
                    entry.0 = entry.0.min(idx);
                    entry.1 = entry.1.max(idx);
                    map
                },
            )
            .into_iter()
            .filter(|(key, ..)| key.weekday() != Weekday::Sat && key.weekday() != Weekday::Sun)
            .map(|(key, (min, max))| (key, max - min))
            .filter(|(_, diff)| *diff >= min_working_hours)
            .map(|(key, _)| key)
            .sorted()
            .collect::<Vec<_>>();
        if work_days.len() <= days {
            return self.clone();
        }

        let days_to_keep = &work_days[(work_days.len() - days)..];
        let min_day = days_to_keep[0];
        self.filtered(|_, idx| idx.date() >= min_day)
    }

    pub fn filtered(&self, filter: impl Fn(usize, NaiveDateTime) -> bool) -> Self {
        let mut df = DataFrame::from_cols(&self.col_names);
        self.index
            .iter()
            .enumerate()
            .filter(|(i, idx)| filter(*i, **idx))
            .for_each(|(i, &idx)| {
                df.index.push(idx);
                for col in &self.col_names {
                    let column = df.columns.get_mut(col).unwrap();
                    column.push(self.columns[col][i]);
                }
            });
        df
    }

    pub fn json(&self) -> Value {
        let mut rows = Vec::with_capacity(self.index.len());
        for i in 0..self.index().len() {
            let mut row = Map::with_capacity(self.col_names.len() + 1);
            row.insert(
                "time".to_owned(),
                Value::from(self.index[i].and_utc().timestamp()),
            );
            for col in &self.col_names {
                let v = self.columns[col][i];
                row.insert(
                    col.to_owned(),
                    if v.is_nan() {
                        Value::Null
                    } else {
                        Value::from(v)
                    },
                );
            }
            rows.push(Value::Object(row));
        }
        Value::Array(rows)
    }

    fn column_widths(&self) -> Vec<usize> {
        let mut widths = Vec::new();
        widths.push(
            self.index
                .iter()
                .map(|t| t.to_string().len())
                .max()
                .unwrap_or(0)
                + 2,
        );
        for name in &self.col_names {
            let width = self.columns[name]
                .iter()
                .map(|v| format!("{v:.2}").len())
                .max()
                .unwrap_or(0)
                + 2;
            widths.push(name.len().max(width));
        }
        widths
    }

    fn create_row(&self, idx: usize, widths: &[usize]) -> String {
        let mut row = String::from("|");
        write!(row, " {:<width$} |", self.index[idx], width = widths[0] - 2).unwrap();
        for (i, name) in self.col_names.iter().enumerate() {
            let value = format!("{:.2}", self.columns[name][idx]);
            write!(row, " {:>width$} |", value, width = widths[i + 1] - 2).unwrap();
        }
        row
    }
}

impl<T> Index<T> for DataFrame
where
    T: AsRef<str>,
{
    type Output = Vec<f64>;

    fn index(&self, index: T) -> &Self::Output {
        let col = index.as_ref();
        self.columns.get(col).unwrap_or_else(|| {
            panic!(
                "Column {} not found, available columns: {:?}",
                col, self.col_names
            )
        })
    }
}

impl Debug for DataFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        print(self, f, true)
    }
}

impl Display for DataFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        print(self, f, false)
    }
}

fn print(df: &DataFrame, f: &mut std::fmt::Formatter, debug: bool) -> std::fmt::Result {
    writeln!(f, "Shape: {:?}", df.shape())?;
    let len = df.shape().0;
    if len == 0 {
        return Ok(());
    }

    let headers = df.column_names();
    let widths = df.column_widths();
    let separator_line = create_separator(&widths);

    writeln!(f, "{separator_line}")?;
    writeln!(f, "{}", create_header_row(&headers, &widths))?;
    writeln!(f, "{separator_line}")?;
    if debug || len <= 10 {
        for i in 0..df.index.len() {
            writeln!(f, "{}", df.create_row(i, &widths))?;
        }
    } else {
        for i in 0..5 {
            writeln!(f, "{}", df.create_row(i, &widths))?;
        }
        writeln!(f, "{}", create_ellipsis_row(&widths))?;
        for i in (len - 5)..len {
            writeln!(f, "{}", df.create_row(i, &widths))?;
        }
    }

    writeln!(f, "{separator_line}")
}

fn create_separator(widths: &[usize]) -> String {
    let mut separator = String::from("+");
    for &width in widths {
        write!(separator, "{}", "-".repeat(width)).unwrap();
        write!(separator, "+").unwrap();
    }
    separator
}

fn create_header_row(headers: &[String], widths: &[usize]) -> String {
    let mut row = String::from("|");
    for (i, header) in headers.iter().enumerate() {
        write!(row, " {:^width$} |", header, width = widths[i] - 2).unwrap();
    }
    row
}

fn create_ellipsis_row(widths: &[usize]) -> String {
    let mut row = String::from("|");
    for (i, &width) in widths.iter().enumerate() {
        if i == 0 {
            write!(row, " {:<width$} |", "...", width = width - 2).unwrap();
        } else {
            write!(row, " {:>width$} |", "...", width = width - 2).unwrap();
        }
    }
    row
}
