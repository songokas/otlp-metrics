use core::time::Duration;
use std::{
    sync::{Arc, Mutex},
    vec,
};

use metrics::{Counter, Gauge, Histogram, Key, KeyName, Metadata, Recorder, SharedString, Unit};

use crate::{
    json,
    metric::{
        CounterValue, GaugeValue, HistogramValue, MetricData, MetricDescription, MetricType,
        MetricValues,
    },
    time::current_time,
};

macro_rules! return_existing_metric {
    ($self:ident, $key:ident, $mtype:ident) => {
        if let Some(value) = $self
            .metrics
            .lock()
            .expect("metrics lock")
            .iter()
            .find(|(k, _)| k.name() == $key.name())
            .map(|(_, v)| match &v.metric_type {
                MetricType::$mtype(v) => v.clone(),
                v => panic!("Unexpected metric type {v} expected $mtype"),
            })
        {
            return $mtype::from_arc(value);
        }
    };
}

#[derive(Default)]
pub struct OtlpRecorder {
    name: String,
    version: String,
    instance_id: String,
    metrics: Mutex<MetricValues>,
    descriptions: Mutex<Vec<MetricDescription>>,
}

impl OtlpRecorder {
    pub fn new(name: impl ToString, version: impl ToString, instance_id: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            instance_id: instance_id.to_string(),
            metrics: Default::default(),
            descriptions: Default::default(),
        }
    }

    pub fn to_json(&self, period: Option<Duration>) -> String {
        let metrics = self.metrics.lock().expect("metrics lock");

        let metrics_to_output: Vec<&(Key, MetricData)> = if let Some(p) = period {
            metrics
                .iter()
                .filter(|(_, m)| match &m.metric_type {
                    MetricType::Counter(v) => current_time() - v.time() <= p.as_nanos() as u64,
                    MetricType::Gauge(v) => current_time() - v.time() <= p.as_nanos() as u64,
                    MetricType::Histogram(v) => current_time() - v.time() <= p.as_nanos() as u64,
                })
                .collect()
        } else {
            metrics.iter().collect::<Vec<&(Key, MetricData)>>()
        };
        json::metrics_to_json(
            &self.name,
            &self.version,
            &self.instance_id,
            metrics_to_output.as_slice(),
        )
    }

    fn update_description(&self, key: &str, metric: &mut MetricData) {
        if let Some(d) = self
            .descriptions
            .lock()
            .expect("description lock")
            .iter()
            .find(|d| d.key.as_str() == key)
        {
            metric.description = d.description.clone();
            metric.unit = d.unit;
        }
    }

    fn add_description(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.descriptions
            .lock()
            .expect("metrics lock")
            .push(MetricDescription {
                key,
                description,
                unit,
            });
    }

    fn add_metric(&self, key: Key, mut metric: MetricData) {
        self.update_description(key.name(), &mut metric);

        self.metrics
            .lock()
            .expect("metrics lock")
            .push((key, metric));
    }
}

impl Recorder for OtlpRecorder {
    fn describe_counter(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.add_description(key, unit, description);
    }

    fn describe_gauge(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.add_description(key, unit, description);
    }

    fn describe_histogram(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.add_description(key, unit, description);
    }

    fn register_counter(&self, key: &Key, _metadata: &Metadata<'_>) -> Counter {
        return_existing_metric!(self, key, Counter);

        let value = Arc::new(CounterValue::default());
        let metric = MetricData::basic(MetricType::Counter(value.clone()));

        self.add_metric(key.clone(), metric);

        Counter::from_arc(value)
    }

    fn register_gauge(&self, key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        return_existing_metric!(self, key, Gauge);

        let value = Arc::new(GaugeValue::default());
        let metric = MetricData::basic(MetricType::Gauge(value.clone()));

        self.add_metric(key.clone(), metric);

        Gauge::from_arc(value)
    }

    fn register_histogram(&self, key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        return_existing_metric!(self, key, Histogram);

        let key = key.clone();

        let bounds = if let Some(buckets) = key
            .labels()
            .find_map(|l| (l.key() == "buckets").then_some(l.value()))
        {
            buckets
                .split(',')
                .map(|v| {
                    v.trim()
                        .parse()
                        .unwrap_or_else(|_| panic!("Invalid value for bucket provided {v}"))
                })
                .collect()
        } else {
            vec![]
        };

        let value = Arc::new(HistogramValue::from_bounds(bounds));
        let metric = MetricData::basic(MetricType::Histogram(value.clone()));

        self.add_metric(key, metric);

        Histogram::from_arc(value)
    }
}
