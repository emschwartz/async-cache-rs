mod async_cache;
mod sync_cache;

use async_cache::AsyncCache;
use chrono::Duration;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Response {
    Square {
        msg: u32,
        #[serde(alias = "ttl(ms)")]
        ttl: u32,
    },
    Error {
        error: String,
    },
}

#[tokio::main]
async fn main() {
    let cache = AsyncCache::new();
    let square_cache = cache.cache_fn(|num: i32| async move {
        let response = reqwest::get(format!("http://localhost:5000/squareme?num={}", num))
            .await
            .unwrap()
            .json::<Response>()
            .await
            .unwrap();
        match response {
            Response::Square { msg, ttl } => Ok((msg, Duration::milliseconds(ttl as i64))),
            Response::Error { error } => Err(error),
        }
    });
    println!("the square of {} is {:?}", 30, square_cache(30).await);
    println!("the square of {} is {:?}", 30, square_cache(30).await);
}
