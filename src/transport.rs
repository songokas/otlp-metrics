use core::time::Duration;
use std::{
    io::{self, Read, Result, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::Arc,
    thread::{sleep, spawn},
};

use tracing::error;

use crate::otlp_recorder::OtlpRecorder;

pub struct TransportConfig {
    pub remote_addr: String,
    pub endpoint: String,
    pub headers: Vec<(String, String)>,
    pub timeout: Duration,
}

/// Send metrics to opentelemetry receiver
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
/// use otlp_metrics::install_recorder;
/// use otlp_metrics::transport::{TransportConfig, send_metrics};
/// use metrics::{counter, gauge, histogram};
///
/// let recorder = install_recorder(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
/// counter!("test_counter", "label1" => "label_value1").increment(1);
/// let config = TransportConfig {
///    remote_addr: "127.0.0.1:9090".to_string(),
///    endpoint: "/api/v1/otlp/v1/metrics".to_string(),
///    headers: vec![("Authorization".to_string(), "Basic ame".to_string())],
///    timeout: Duration::from_secs(5),
/// };
/// let response = send_metrics(&config, recorder.to_json().as_bytes()).unwrap();
/// ```
pub fn send_metrics(config: &TransportConfig, metrics: &[u8]) -> Result<Vec<u8>> {
    let TransportConfig {
        remote_addr,
        endpoint,
        headers,
        timeout,
    } = config;
    let Some(addr) = remote_addr.to_socket_addrs()?.next() else {
        return Err(io::Error::other("Socket address unknown"));
    };

    let mut stream = TcpStream::connect_timeout(&addr, *timeout)?;

    let Some(host) = remote_addr.split(':').next() else {
        return Err(io::Error::other("Host address unknown"));
    };
    let mut request =
        format!("POST {endpoint} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n", metrics.len());
    for (k, v) in headers {
        request.push_str(&format!("{k}: {v}\r\n"))
    }
    request.push_str("\r\n");

    stream.write_all(request.as_bytes())?;
    stream.write_all(metrics)?;
    stream.flush()?;
    let mut response = vec![0; 200];
    let _ = stream.read(&mut response)?;
    Ok(response)
}

/// Spawn a thread that sends metrics to opentelemetry receiver at specific intervals
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
/// use otlp_metrics::install_recorder;
/// use otlp_metrics::transport::{TransportConfig, send_metrics_with_interval};
/// use metrics::{counter, gauge, histogram};
///
/// let recorder = install_recorder(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
/// counter!("test_counter", "label1" => "label_value1").increment(1);
/// let config = TransportConfig {
///    remote_addr: "127.0.0.1:9090".to_string(),
///    endpoint: "/api/v1/otlp/v1/metrics".to_string(),
///    headers: vec![("Authorization".to_string(), "Basic ame".to_string())],
///    timeout: Duration::from_secs(5),
/// };
/// send_metrics_with_interval(config, Duration::from_secs(15), recorder);
/// ```
pub fn send_metrics_with_interval(
    config: TransportConfig,
    interval: Duration,
    recorder: Arc<OtlpRecorder>,
) {
    spawn(move || loop {
        sleep(interval);
        if let Err(e) = send_metrics(&config, recorder.to_json().as_bytes()) {
            error!("Error sending metrics {e}");
        }
    });
}

#[cfg(test)]
mod tests {
    use metrics::{counter, gauge, histogram};

    use crate::{install_recorder, time::set_time};

    use super::*;

    // docker run -p 9090:9090 -v $(pwd)/tests/prometheus.yml:/etc/prometheus/prometheus.yml prom/prometheus --web.enable-otlp-receiver --config.file=/etc/prometheus/prometheus.yml
    #[test]
    fn test_send_metrics() {
        set_time(0);
        sleep(Duration::from_millis(1000));
        let recorder = install_recorder("otlp-metrics", "0.1.0");
        for _ in 0..3 {
            counter!("test_counter", "label1" => "label_value1").increment(1);
            gauge!("test_gauge", "label2" => "label_value2").set(10);
            histogram!("test_histogram", "label3" => "label_value3").record(10);
        }
        let config = TransportConfig {
            remote_addr: "127.0.0.1:9090".to_string(),
            endpoint: "/api/v1/otlp/v1/metrics".to_string(),
            headers: vec![("Authorization".to_string(), "Basic ame".to_string())],
            timeout: Duration::from_secs(5),
        };
        let response = send_metrics(&config, recorder.to_json().as_bytes()).unwrap();
        assert!(String::from_utf8(response)
            .unwrap()
            .contains("HTTP/1.1 200 OK"));
        for _ in 0..3 {
            counter!("test_counter", "label1" => "label_value1").increment(1);
            gauge!("test_gauge", "label2" => "label_value2").set(10);
            histogram!("test_histogram", "label3" => "label_value3").record(10);
        }
        let response = send_metrics(&config, recorder.to_json().as_bytes()).unwrap();
        assert!(String::from_utf8(response)
            .unwrap()
            .contains("HTTP/1.1 200 OK"));
    }
}
