use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Tick {
    // bid queue
    bids: Vec<Vec<f64>>,
    // ask queue
    asks: Vec<Vec<f64>>,
    // api version
    version: i64,
    // timestamp
    ts: i64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Quote {
    ch: String,
    ts: i64,
    tick: Tick
}