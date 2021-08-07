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

struct SquareCache {
    cache: AsyncCache<i32, u32>,
}

impl SquareCache {
    pub fn new() -> Self {
        SquareCache {
            cache: AsyncCache::new(),
        }
    }

    pub async fn square(&self, num: i32) -> Result<u32, String> {
        if let Some(square) = self.cache.get(&num).await {
            println!("got value {} from cache", square);
            Ok(square)
        } else {
            let response = reqwest::get(format!("http://localhost:5000/squareme?num={}", num))
                .await
                .unwrap()
                .json::<Response>()
                .await
                .unwrap();

            match response {
                Response::Square { msg, ttl } => {
                    self.cache
                        .set(num, msg, Duration::milliseconds(ttl as i64))
                        .await;
                    Ok(msg)
                }
                Response::Error { error } => Err(error),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let square_cache = SquareCache::new();
    println!(
        "the square of {} is {}",
        30,
        square_cache.square(30).await.unwrap()
    );
    println!(
        "the square of {} is {}",
        30,
        square_cache.square(30).await.unwrap()
    );
}
