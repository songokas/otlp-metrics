use json::{object, JsonValue};
use metrics::Key;

use crate::metric::{
    CounterValue, GaugeValue, HistogramValue, MetricData, MetricType, MetricValues,
};

pub fn metrics_to_json(name: &str, version: &str, values: &MetricValues) -> String {
    let value = root(name, version, values);
    json::stringify(value)
}

fn root(name: &str, version: &str, values: &MetricValues) -> JsonValue {
    object! {
        "resourceMetrics": [{
            "resource": {
                "attributes": [
                    attr(name, version)
                ]
            },
            "scopeMetrics": [{
                "metrics": values.iter().map(|(k, v)| {
                    match &v.metric_type {
                        MetricType::Counter(m) => counter(k, v, m),
                        MetricType::Gauge(m) => gauge(k, v, m),
                        MetricType::Histogram(m) => histogram(k, v, m),
                    }
                }).collect::<Vec<_>>(),
            }]
        }]
    }
}

fn counter(key: &Key, data: &MetricData, value: &CounterValue) -> JsonValue {
    object! {
        "name": key.name(),
        "unit": data.unit(),
        "description": data.description.to_string(),
        "sum": {
            "aggregationTemporality": 2,
            "isMonotonic": true,
            "dataPoints": [
                {
                    "asInt": value.value(),
                    "startTimeUnixNano": data.start_time,
                    "timeUnixNano": value.time(),
                    "attributes": key.labels().map(|l| attr(l.key(), l.value())).collect::<Vec<_>>()
                }
            ]
        }
    }
}

fn gauge(key: &Key, data: &MetricData, value: &GaugeValue) -> JsonValue {
    object! {
        "name": key.name(),
        "unit": data.unit(),
        "description": data.description.to_string(),
        "gauge": {
            "dataPoints": [
                {
                    "asDouble": value.value(),
                    "startTimeUnixNano": data.start_time,
                    "timeUnixNano": value.time(),
                    "attributes": key.labels().map(|l| attr(l.key(), l.value())).collect::<Vec<_>>()
                }
            ]
        }
    }
}

fn histogram(key: &Key, data: &MetricData, value: &HistogramValue) -> JsonValue {
    object! {
        "name": key.name(),
        "unit": data.unit(),
        "description": data.description.to_string(),
        "histogram": {
            "aggregationTemporality": 2,
            "dataPoints": [
                {
                    "startTimeUnixNano": data.start_time,
                    "timeUnixNano": value.time(),
                    "count": value.count(),
                    "sum": value.sum(),
                    "attributes": key.labels().map(|l| attr(l.key(), l.value())).collect::<Vec<_>>(),
                    "bucketCounts": value.bucket_count(),
                    "explicitBounds": value.explicit_bounds(),
                }
            ]
        }
    }
}

fn attr(key: &str, value: &str) -> JsonValue {
    object! {
        "key": key,
        "value": {
            "stringValue": value
        }
    }
}
