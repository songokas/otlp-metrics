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
    pub explicit_bounds: Vec<f64>,
    pub bucket_count: Vec<AtomicU64>,
}

impl HistogramValue {
    pub fn from_bounds(bounds: Vec<f64>) -> Self {
        let mut value = Self::default();
        if !bounds.is_empty() {
            value.explicit_bounds = bounds;
            value.bucket_count = value
                .explicit_bounds
                .iter()
                .map(|_| AtomicU64::new(0))
                .collect();
            value.bucket_count.push(AtomicU64::new(0));
        }
        value
    }

    pub fn sum(&self) -> f64 {
        f64::from_bits(self.sum.load(Ordering::Relaxed))
    }

    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn time(&self) -> u64 {
        self.time.load(Ordering::Relaxed)
    }

    pub fn bucket_count(&self) -> Vec<u64> {
        self.bucket_count
            .iter()
            .map(|v| v.load(Ordering::Relaxed))
            .collect()
    }

    pub fn explicit_bounds(&self) -> &[f64] {
        &self.explicit_bounds
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

        if !self.explicit_bounds.is_empty() {
            let mut bounds = self.explicit_bounds.iter();
            let mut buckets = self.bucket_count.iter();
            let mut prev_bound = bounds.next().expect("At least one bound");
            let bucket = buckets.next().expect("At least one bucket");
            if &value <= prev_bound {
                let _ = bucket.fetch_add(1, Ordering::Release);
            }
            loop {
                let bound = bounds.next();
                let bucket = buckets.next();
                if let Some(b) = bound {
                    if &value > prev_bound && &value <= b {
                        let _ = bucket
                            .expect("At least one bound")
                            .fetch_add(1, Ordering::Release);
                    }
                    prev_bound = b;
                } else {
                    if &value > prev_bound {
                        let _ = bucket
                            .expect("At least one bound")
                            .fetch_add(1, Ordering::Release);
                    }
                    break;
                }
            }
        }

        let _ = self.count.fetch_add(1, Ordering::Release);
        let _ = self.time.swap(current_time(), Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_bounds() {
        let histogram = HistogramValue::from_bounds(vec![1.0, 2.0, 100.0]);
        histogram.record(-1.0);
        assert_eq!(histogram.bucket_count(), vec![1, 0, 0, 0]);
        histogram.record(1.0);
        assert_eq!(histogram.bucket_count(), vec![2, 0, 0, 0]);
        histogram.record(1.5);
        assert_eq!(histogram.bucket_count(), vec![2, 1, 0, 0]);
        histogram.record(2.5);
        assert_eq!(histogram.bucket_count(), vec![2, 1, 1, 0]);
        histogram.record(100.0);
        assert_eq!(histogram.bucket_count(), vec![2, 1, 2, 0]);
        histogram.record(1000.0);
        assert_eq!(histogram.bucket_count(), vec![2, 1, 2, 1]);

        assert_eq!(histogram.count(), 6);
        assert_eq!(histogram.sum(), 1104.0);
    }

    #[test]
    fn test_gauge() {
        let value = GaugeValue::default();
        value.set(-10.0);
        assert_eq!(value.value(), -10.0);
        value.set(10.0);
        assert_eq!(value.value(), 10.0);
    }

    #[test]
    fn test_counter() {
        let value = CounterValue::default();
        value.increment(1);
        assert_eq!(value.value(), 1);
        value.increment(100);
        assert_eq!(value.value(), 101);
    }
}
