use chrono::{Timelike, Utc};
use lazy_static::lazy_static;
use log::error;
use prometheus::{Gauge, IntGauge, Registry};
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
use std::env;
use std::net::Ipv4Addr;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use warp::{Filter, Rejection, Reply};

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_BIND_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    static ref LATITUDE: Gauge =
        Gauge::new("latitude", "latitude").expect("latitude can be created");
    static ref LONGITUDE: Gauge =
        Gauge::new("longitude", "longitude").expect("longitude can be created");
    static ref WIND_SPEED: Gauge =
        Gauge::new("wind_speed", "wind speed").expect("wind_speed can be created");
    static ref WIND_DIRECTION: IntGauge =
        IntGauge::new("wind_direction", "wind direction").expect("wind_direction can be created");
    static ref PORT: u16 = {
        if let Some(port) = env::var_os("PORT") {
            port.into_string().unwrap().parse::<u16>().unwrap()
        } else {
            DEFAULT_PORT
        }
    };
    static ref BIND_ADDR: Ipv4Addr = {
        if let Some(addr) = env::var_os("BIND_ADDR") {
            addr.into_string().unwrap().parse::<Ipv4Addr>().unwrap()
        } else {
            DEFAULT_BIND_ADDR
        }
    };
}

pub struct MetricsProvider {
    webserver_thread: Option<JoinHandle<()>>,
    data_generator_thread: Option<JoinHandle<()>>,
}

impl MetricsProvider {
    pub fn new() -> Self {
        REGISTRY
            .register(Box::new(LATITUDE.clone()))
            .expect("LATITUDE can be registered");
        REGISTRY
            .register(Box::new(LONGITUDE.clone()))
            .expect("LONGITUDE can be registered");
        REGISTRY
            .register(Box::new(WIND_SPEED.clone()))
            .expect("WIND_SPEED can be registered");
        REGISTRY
            .register(Box::new(WIND_DIRECTION.clone()))
            .expect("WIND_DIRECTION can be registered");

        MetricsProvider {
            webserver_thread: None,
            data_generator_thread: None,
        }
    }

    pub fn run(&mut self) {
        self.webserver_thread = Some(tokio::spawn(async move {
            warp::serve(warp::path!("metrics").and_then(MetricsProvider::metrics_handler))
                .run((BIND_ADDR.octets(), *PORT))
                .await;
        }));

        self.data_generator_thread = Some(tokio::spawn(MetricsProvider::data_collector()));
    }

    pub async fn stop(&mut self) {
        if self.webserver_thread.is_some() {
            self.webserver_thread.as_ref().unwrap().abort();
        }

        if self.data_generator_thread.is_some() {
            self.data_generator_thread.as_ref().unwrap().abort();
        }
    }

    async fn data_collector() {
        // configure interval of wind speed and wind direction samples in seconds
        let mut collect_interval = tokio::time::interval(Duration::from_secs(10));

        let wind_speed_range: Uniform<f64> = Uniform::new_inclusive(0.0, 10.0);
        let wind_direction_range: Uniform<i64> = Uniform::new_inclusive(0, 359);
        let wind_speed_per_hour: Vec<f64> = thread_rng()
            .sample_iter(wind_speed_range)
            .take(24)
            .collect();
        let wind_direction_per_hour: Vec<i64> = thread_rng()
            .sample_iter(wind_direction_range)
            .take(24)
            .collect();

        // gen location
        LATITUDE.set(thread_rng().gen_range(-85.05112878..85.05112878));
        LONGITUDE.set(thread_rng().gen_range(-180.0..180.0));

        loop {
            // get wind speed of current hour
            // apply random deviation of -5.0 to 5.0 percent
            match wind_speed_per_hour.get(Utc::now().hour() as usize) {
                Some(v) => WIND_SPEED.set(v + (v * thread_rng().gen_range(-5.0..5.0) / 100.0)),
                _ => error!("Couldn't generate wind speed"),
            }

            // get wind direction of current hour
            // apply random deviation of -5.0 to 5.0 percent
            match wind_direction_per_hour.get(Utc::now().hour() as usize) {
                Some(v) => WIND_DIRECTION
                    .set(v + (*v as f64 * thread_rng().gen_range(-5.0..5.0) / 100.0) as i64),
                _ => error!("Couldn't generate wind direction"),
            }

            collect_interval.tick().await;
        }
    }

    async fn metrics_handler() -> Result<impl Reply, Rejection> {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();

        let mut buffer = Vec::new();
        if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
            eprintln!("could not encode custom metrics: {}", e);
        };
        let mut res = match String::from_utf8(buffer.clone()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("custom metrics could not be from_utf8'd: {}", e);
                String::default()
            }
        };

        buffer.clear();

        let mut buffer = Vec::new();
        if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
            eprintln!("could not encode prometheus metrics: {}", e);
        };
        let res_custom = match String::from_utf8(buffer.clone()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("prometheus metrics could not be from_utf8'd: {}", e);
                String::default()
            }
        };
        buffer.clear();

        res.push_str(&res_custom);
        Ok(res)
    }
}
