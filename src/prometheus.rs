use chrono::{Timelike, Utc};
use lazy_static::lazy_static;
use prometheus::{Gauge, IntGauge, Registry};
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
use tokio::time::Duration;
use warp::{Filter, Rejection, Reply};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref LATITUDE: Gauge =
        Gauge::new("latitude", "latitude").expect("metric can be created");
    pub static ref LONGITUDE: Gauge =
        Gauge::new("longitude", "longitude").expect("metric can be created");
    pub static ref WIND_SPEED: Gauge =
        Gauge::new("wind_speed", "wind speed").expect("metric can be created");
    pub static ref WIND_DIRECTION: IntGauge =
        IntGauge::new("wind_direction", "wind direction").expect("metric can be created");
}

pub fn run() {
    //add call_once
    REGISTRY
        .register(Box::new(LATITUDE.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(LONGITUDE.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(WIND_SPEED.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(WIND_DIRECTION.clone()))
        .expect("collector can be registered");

    tokio::spawn(async move {
        warp::serve(warp::path!("metrics").and_then(metrics_handler))
            .run(([0, 0, 0, 0], 8080))
            .await;
    });

    tokio::spawn(data_collector());
}

/* pub async fn stop() {

    abort
} */

async fn data_collector() {
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
        // create random deviation between -5.0 and 5.0 percent
        let percent: f64 = thread_rng().gen_range(-5.0..5.0);

        // get current wind speed and apply deviation
        let wind_speed = wind_speed_per_hour.get(Utc::now().hour() as usize).unwrap();
        WIND_SPEED.set(wind_speed + (wind_speed * percent / 100.0));

        // get current wind direction and apply deviation
        let wind_direction = wind_direction_per_hour
            .get(Utc::now().hour() as usize)
            .unwrap();
        WIND_DIRECTION.set(wind_direction + (*wind_direction as f64 * percent / 100.0) as i64);

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
