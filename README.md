# About

Simple opentelemetry metrics exporter as json

# Howto

```rust
use otlp_metrics::install_recorder;
use metrics::{counter, gauge, histogram};
use otlp_metrics::transport::{TransportConfig, send_metrics, send_metrics_with_interval};

let recorder = install_recorder(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

counter!("test_counter", "label1" => "label_value1").increment(1);
gauge!("test_gauge", "label2" => "label_value2").set(10);
histogram!("test_histogram", "label3" => "label_value3").record(10);

let config = TransportConfig {
   remote_addr: "127.0.0.1:9090".to_string(),
   endpoint: "/api/v1/otlp/v1/metrics".to_string(),
   headers: vec![("Authorization".to_string(), "Basic ame".to_string())],
   timeout: Duration::from_secs(5),
};

// send metrics manually
let response = send_metrics(&config, recorder.to_json().as_bytes())?;

// send metrics every 15 seconds
send_metrics_with_interval(config, Duration::from_secs(15), recorder);
```

src/lib.rs
