use actix_web::{web, App, HttpServer, Responder};
use chrono::{Timelike, Utc};
use futures_executor::block_on;
use lazy_static::lazy_static;
use log::{error, info};
use prometheus::{Gauge, IntGauge, Registry};
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
use std::env;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::task::JoinHandle;
use tokio::time::Duration;

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
    static ref ADDR: SocketAddr = {
        let def = SocketAddr::from(([0, 0, 0, 0], 8080));
        let addr = env::var_os("BIND_ADDR_AND_PORT").unwrap_or(def.to_string().into());

        addr.into_string()
            .unwrap_or(def.to_string())
            .to_socket_addrs()
            .unwrap_or(vec![def].into_iter())
            .next()
            .unwrap_or(def)
    };
}

pub struct MetricsProvider {
    threads: Vec<Option<JoinHandle<()>>>,
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

        MetricsProvider { threads: vec![] }
    }

    pub fn run(&mut self, location: serde_json::Value) {
        self.threads.push(Some(tokio::spawn(async move {
            block_on(async move {
                let _ = HttpServer::new(|| {
                    App::new().route("/metrics", web::get().to(Self::metrics_handler))
                })
                .bind(*ADDR)
                .unwrap()
                .run()
                .await;
            })
        })));

        self.threads
            .push(Some(tokio::spawn(MetricsProvider::data_collector(
                location["latitude"].as_f64().unwrap(),
                location["longitude"].as_f64().unwrap(),
            ))));
    }

    pub fn stop(&mut self) {
        for thread in self.threads.iter_mut() {
            if thread.is_some() {
                let thread = thread.as_mut().unwrap();
                thread.abort();
                block_on(async {
                    thread.await.unwrap_or_else(|e| {
                        if !e.is_cancelled() {
                            error!("thread terminated with error: {}", e.to_string());
                        }
                    });
                });
            }
        }
    }

    async fn data_collector(latitude: f64, longitude: f64) {
        // configure interval of wind speed and wind direction samples in seconds
        let mut collect_interval = tokio::time::interval(Duration::from_secs(10));

        // init simulation ranges
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

        // set location
        info!("set LATITUDE: {} LONGITUDE: {}", latitude, longitude);
        LATITUDE.set(latitude);
        LONGITUDE.set(longitude);

        loop {
            // get wind speed of current hour
            // apply random deviation of -5.0 to 5.0 percent
            match wind_speed_per_hour.get(Utc::now().hour() as usize) {
                Some(v) => WIND_SPEED.set(v + (v * thread_rng().gen_range(-5.0..5.0) / 100.0)),
                _ => error!("couldn't generate wind speed"),
            }

            // get wind direction of current hour
            // apply random deviation of -5.0 to 5.0 percent
            match wind_direction_per_hour.get(Utc::now().hour() as usize) {
                Some(v) => WIND_DIRECTION
                    .set(v + (*v as f64 * thread_rng().gen_range(-5.0..5.0) / 100.0) as i64),
                _ => error!("couldn't generate wind direction"),
            }

            collect_interval.tick().await;
        }
    }

    async fn metrics_handler() -> impl Responder {
        use prometheus::Encoder;

        let encoder = prometheus::TextEncoder::new();
        let mut buffer = Vec::new();

        if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
            error!("could not encode custom metrics: {}", e);
        };

        let mut res = match String::from_utf8(buffer.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("custom metrics could not be from_utf8'd: {}", e);
                String::default()
            }
        };

        buffer.clear();

        let mut buffer = Vec::new();

        if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
            error!("could not encode prometheus metrics: {}", e);
        };

        let res_custom = match String::from_utf8(buffer.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("prometheus metrics could not be from_utf8'd: {}", e);
                String::default()
            }
        };

        buffer.clear();

        res.push_str(&res_custom);

        res
    }
}

impl Drop for MetricsProvider {
    fn drop(&mut self) {
        self.stop();
    }
}
