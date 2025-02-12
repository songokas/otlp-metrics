use core::{
    fmt::Display,
    sync::atomic::{AtomicU64, Ordering},
};
use std::sync::Arc;

use metrics::{CounterFn, GaugeFn, HistogramFn, Key, KeyName, SharedString, Unit};

use crate::time::current_time;

pub type MetricValues = Vec<(Key, MetricData)>;

pub enum MetricType {
    Counter(Arc<CounterValue>),
    Gauge(Arc<GaugeValue>),
    Histogram(Arc<HistogramValue>),
}

impl Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::Counter(_metadata) => write!(f, "counter"),
            MetricType::Gauge(_) => write!(f, "gauge"),
            MetricType::Histogram(_) => write!(f, "histogram"),
        }
    }
}

pub struct MetricDescription {
    pub key: KeyName,
    pub description: SharedString,
    pub unit: Option<Unit>,
}

pub struct MetricData {
    pub start_time: u64,
    pub description: SharedString,
    pub unit: Option<Unit>,
    pub metric_type: MetricType,
}

impl MetricData {
    pub fn basic(metric_type: MetricType) -> Self {
        Self {
            unit: None,
            start_time: current_time(),
            description: SharedString::default(),
            metric_type,
        }
    }

    pub fn unit(&self) -> &str {
        self.unit.map(|u| u.as_canonical_label()).unwrap_or("1")
    }
}

#[derive(Default)]
pub struct CounterValue {
    pub value: AtomicU64,
    pub time: AtomicU64,
}

impl CounterValue {
    pub fn value(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn time(&self) -> u64 {
        self.time.load(Ordering::Relaxed)
    }
}

impl CounterFn for CounterValue {
    fn increment(&self, value: u64) {
        let _ = self.value.fetch_add(value, Ordering::Release);
        let _ = self.time.swap(current_time(), Ordering::AcqRel);
    }

    fn absolute(&self, value: u64) {
        let _ = self.value.fetch_max(value, Ordering::AcqRel);
        let _ = self.time.swap(current_time(), Ordering::AcqRel);
    }
}

#[derive(Default)]
pub struct GaugeValue {
    pub value: AtomicU64,
    pub time: AtomicU64,
}

impl GaugeValue {
    pub fn value(&self) -> f64 {
        f64::from_bits(self.value.load(Ordering::Relaxed))
    }

    pub fn time(&self) -> u64 {
        self.time.load(Ordering::Relaxed)
    }
}

impl GaugeFn for GaugeValue {
    fn increment(&self, value: f64) {
        loop {
            let result = self
                .value
                .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |curr| {
                    let input = f64::from_bits(curr);
                    let output = input + value;
                    Some(output.to_bits())
                });

            if result.is_ok() {
                break;
            }
        }
        let _ = self.time.swap(current_time(), Ordering::AcqRel);
    }

    fn decrement(&self, value: f64) {
        loop {
            let result = self
                .value
                .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |curr| {
                    let input = f64::from_bits(curr);
                    let output = input - value;
                    Some(output.to_bits())
                });

            if result.is_ok() {
                break;
            }
        }
        let _ = self.time.swap(current_time(), Ordering::AcqRel);
    }

    fn set(&self, value: f64) {
        let _ = self.value.swap(value.to_bits(), Ordering::AcqRel);
        let _ = self.time.swap(current_time(), Ordering::AcqRel);
    }
}

#[derive(Default)]
pub struct HistogramValue {
    pub sum: AtomicU64,
    pub count: AtomicU64,
    pub time: AtomicU64,
}

impl HistogramValue {
    pub fn sum(&self) -> f64 {
        f64::from_bits(self.sum.load(Ordering::Relaxed))
    }

    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn time(&self) -> u64 {
        self.time.load(Ordering::Relaxed)
    }
}

impl HistogramFn for HistogramValue {
    fn record(&self, value: f64) {
        loop {
            let result = self
                .sum
                .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |curr| {
                    let input = f64::from_bits(curr);
                    let output = input + value;
                    Some(output.to_bits())
                });

            if result.is_ok() {
                break;
            }
        }

        let _ = self.count.fetch_add(1, Ordering::Release);
        let _ = self.time.swap(current_time(), Ordering::AcqRel);
    }
}
