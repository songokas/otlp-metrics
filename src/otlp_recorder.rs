use std::sync::{Arc, Mutex};

use metrics::{Counter, Gauge, Histogram, Key, KeyName, Metadata, Recorder, SharedString, Unit};

use crate::{
    json,
    metric::{
        CounterValue, GaugeValue, HistogramValue, MetricData, MetricDescription, MetricType,
        MetricValues,
    },
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
    metrics: Mutex<MetricValues>,
    descriptions: Mutex<Vec<MetricDescription>>,
}

impl OtlpRecorder {
    pub fn new(name: impl ToString, version: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            metrics: Default::default(),
            descriptions: Default::default(),
        }
    }

    pub fn to_json(&self) -> String {
        json::metrics_to_json(
            &self.name,
            &self.version,
            &self.metrics.lock().expect("metrics lock"),
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

        Gauge::from_arc(Arc::new(value))
    }

    fn register_histogram(&self, key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        return_existing_metric!(self, key, Histogram);

        let value = Arc::new(HistogramValue::default());
        let metric = MetricData::basic(MetricType::Histogram(value.clone()));

        self.add_metric(key.clone(), metric);

        Histogram::from_arc(value)
    }
}
