use std::sync::Arc;

use metrics::set_global_recorder;
use otlp_recorder::OtlpRecorder;

mod json;
mod metric;
pub mod otlp_recorder;
mod time;
pub mod transport;

/// Install recorder globally
///
/// # Example
///
/// ```rust
/// use otlp_metrics_exporter::install_recorder;
/// use metrics::{counter, gauge, histogram};
///
/// let recorder = install_recorder(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"), "instance1");
/// counter!("test_counter", "label1" => "label_value1").increment(1);
/// gauge!("test_gauge", "label2" => "label_value2").set(10);
/// histogram!("test_histogram", "label3" => "label_value3").record(10);
/// recorder.to_json(None);
/// ```
pub fn install_recorder(
    name: impl ToString,
    version: impl ToString,
    instance_id: impl ToString,
) -> Arc<OtlpRecorder> {
    let recorder = Arc::new(OtlpRecorder::new(name, version, instance_id));
    set_global_recorder(recorder.clone()).expect("Recorder installed");
    recorder
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use metrics::{
        counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram,
        set_default_local_recorder, Unit,
    };

    use crate::time::set_time;

    use super::*;

    #[test]
    fn test_recorder_to_json() {
        set_time(1739394449205);
        let recorder = OtlpRecorder::new("otlp-metrics", "1", "test_recorder_to_json");
        let _guard = set_default_local_recorder(&recorder);
        for i in 1..3 {
            counter!("test_counter", "label1" => "label_value1").increment(1);
            gauge!("test_gauge", "label2" => "label_value2").set(i * 10);
            histogram!("test_histogram", "label3" => "label_value3").record(i * 10);
            histogram!("test_histogram_with_buckets", "buckets" => "10,30").record(i * 10);
        }

        assert_eq!(
            recorder.to_json(None),
            r#"{"resourceMetrics":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"otlp-metrics"}},{"key":"service.version","value":{"stringValue":"1"}},{"key":"service.instance.id","value":{"stringValue":"test_recorder_to_json"}}]},"scopeMetrics":[{"metrics":[{"name":"test_counter","unit":"1","description":"","sum":{"aggregationTemporality":2,"isMonotonic":true,"dataPoints":[{"asInt":2,"startTimeUnixNano":1739394449305000000,"timeUnixNano":1739394450105000000,"attributes":[{"key":"label1","value":{"stringValue":"label_value1"}}]}]}},{"name":"test_gauge","unit":"1","description":"","gauge":{"dataPoints":[{"asDouble":20,"startTimeUnixNano":1739394449505000000,"timeUnixNano":1739394450205000000,"attributes":[{"key":"label2","value":{"stringValue":"label_value2"}}]}]}},{"name":"test_histogram","unit":"1","description":"","histogram":{"aggregationTemporality":2,"dataPoints":[{"startTimeUnixNano":1739394449705000000,"timeUnixNano":1739394450305000000,"count":2,"sum":30,"attributes":[{"key":"label3","value":{"stringValue":"label_value3"}}],"bucketCounts":[],"explicitBounds":[]}]}},{"name":"test_histogram_with_buckets","unit":"1","description":"","histogram":{"aggregationTemporality":2,"dataPoints":[{"startTimeUnixNano":1739394449905000000,"timeUnixNano":1739394450405000000,"count":2,"sum":30,"attributes":[{"key":"buckets","value":{"stringValue":"10,30"}}],"bucketCounts":[1,1,0],"explicitBounds":[10,30]}]}}]}]}]}"#,
        );
    }

    #[test]
    fn test_recorder_with_descriptions_and_units() {
        set_time(1739394449205);
        let recorder = OtlpRecorder::new(
            "otlp-metrics",
            "1",
            "test_recorder_with_descriptions_and_units",
        );
        let _guard = set_default_local_recorder(&recorder);

        describe_counter!("bytes_total", Unit::Bytes, "Counter for bytes");
        describe_gauge!("limit_reached", Unit::Percent, "Gauge percent");
        describe_histogram!(
            "request_time",
            Unit::Milliseconds,
            "Request time in milliseconds"
        );

        counter!("bytes_total").increment(1);
        gauge!("limit_reached").set(10);
        histogram!("request_time").record(10);

        assert_eq!(
            recorder.to_json(None),
            r#"{"resourceMetrics":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"otlp-metrics"}},{"key":"service.version","value":{"stringValue":"1"}},{"key":"service.instance.id","value":{"stringValue":"test_recorder_with_descriptions_and_units"}}]},"scopeMetrics":[{"metrics":[{"name":"bytes_total","unit":"B","description":"Counter for bytes","sum":{"aggregationTemporality":2,"isMonotonic":true,"dataPoints":[{"asInt":1,"startTimeUnixNano":1739394449305000000,"timeUnixNano":1739394449405000000,"attributes":[]}]}},{"name":"limit_reached","unit":"%","description":"Gauge percent","gauge":{"dataPoints":[{"asDouble":10,"startTimeUnixNano":1739394449505000000,"timeUnixNano":1739394449605000000,"attributes":[]}]}},{"name":"request_time","unit":"ms","description":"Request time in milliseconds","histogram":{"aggregationTemporality":2,"dataPoints":[{"startTimeUnixNano":1739394449705000000,"timeUnixNano":1739394449805000000,"count":1,"sum":10,"attributes":[],"bucketCounts":[],"explicitBounds":[]}]}}]}]}]}"#,
        );
    }

    #[test]
    fn test_metric_times() {
        set_time(1739394449205);
        let recorder = OtlpRecorder::new("otlp-metrics", "1", "test_metric_times");
        let _guard = set_default_local_recorder(&recorder);

        counter!("test_counter", "label1" => "label_value1").increment(1);

        assert_eq!(
            recorder.to_json(None),
            r#"{"resourceMetrics":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"otlp-metrics"}},{"key":"service.version","value":{"stringValue":"1"}},{"key":"service.instance.id","value":{"stringValue":"test_metric_times"}}]},"scopeMetrics":[{"metrics":[{"name":"test_counter","unit":"1","description":"","sum":{"aggregationTemporality":2,"isMonotonic":true,"dataPoints":[{"asInt":1,"startTimeUnixNano":1739394449305000000,"timeUnixNano":1739394449405000000,"attributes":[{"key":"label1","value":{"stringValue":"label_value1"}}]}]}}]}]}]}"#
        );

        counter!("test_counter", "label1" => "label_value1").increment(1);

        assert_eq!(
            recorder.to_json(None),
            r#"{"resourceMetrics":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"otlp-metrics"}},{"key":"service.version","value":{"stringValue":"1"}},{"key":"service.instance.id","value":{"stringValue":"test_metric_times"}}]},"scopeMetrics":[{"metrics":[{"name":"test_counter","unit":"1","description":"","sum":{"aggregationTemporality":2,"isMonotonic":true,"dataPoints":[{"asInt":2,"startTimeUnixNano":1739394449305000000,"timeUnixNano":1739394449505000000,"attributes":[{"key":"label1","value":{"stringValue":"label_value1"}}]}]}}]}]}]}"#
        );
    }

    #[test]
    fn test_output_only_changed_values() {
        set_time(1739394449205);
        let recorder = OtlpRecorder::new("otlp-metrics", "1", "test_output_only_changed_values");
        let _guard = set_default_local_recorder(&recorder);

        counter!("test_counter", "label1" => "label_value1").increment(1);

        assert_eq!(
            recorder.to_json(None),
            r#"{"resourceMetrics":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"otlp-metrics"}},{"key":"service.version","value":{"stringValue":"1"}},{"key":"service.instance.id","value":{"stringValue":"test_output_only_changed_values"}}]},"scopeMetrics":[{"metrics":[{"name":"test_counter","unit":"1","description":"","sum":{"aggregationTemporality":2,"isMonotonic":true,"dataPoints":[{"asInt":1,"startTimeUnixNano":1739394449305000000,"timeUnixNano":1739394449405000000,"attributes":[{"key":"label1","value":{"stringValue":"label_value1"}}]}]}}]}]}]}"#
        );

        assert_eq!(
            recorder.to_json(Duration::from_millis(101).into()),
            r#"{"resourceMetrics":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"otlp-metrics"}},{"key":"service.version","value":{"stringValue":"1"}},{"key":"service.instance.id","value":{"stringValue":"test_output_only_changed_values"}}]},"scopeMetrics":[{"metrics":[{"name":"test_counter","unit":"1","description":"","sum":{"aggregationTemporality":2,"isMonotonic":true,"dataPoints":[{"asInt":1,"startTimeUnixNano":1739394449305000000,"timeUnixNano":1739394449405000000,"attributes":[{"key":"label1","value":{"stringValue":"label_value1"}}]}]}}]}]}]}"#
        );

        assert_eq!(
            recorder.to_json(Duration::from_millis(99).into()),
            r#"{"resourceMetrics":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"otlp-metrics"}},{"key":"service.version","value":{"stringValue":"1"}},{"key":"service.instance.id","value":{"stringValue":"test_output_only_changed_values"}}]},"scopeMetrics":[{"metrics":[]}]}]}"#
        );

        counter!("test_counter", "label1" => "label_value1").increment(1);

        assert_eq!(
            recorder.to_json(Duration::from_secs(99).into()),
            r#"{"resourceMetrics":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"otlp-metrics"}},{"key":"service.version","value":{"stringValue":"1"}},{"key":"service.instance.id","value":{"stringValue":"test_output_only_changed_values"}}]},"scopeMetrics":[{"metrics":[{"name":"test_counter","unit":"1","description":"","sum":{"aggregationTemporality":2,"isMonotonic":true,"dataPoints":[{"asInt":2,"startTimeUnixNano":1739394449305000000,"timeUnixNano":1739394449705000000,"attributes":[{"key":"label1","value":{"stringValue":"label_value1"}}]}]}}]}]}]}"#
        );
    }
}
