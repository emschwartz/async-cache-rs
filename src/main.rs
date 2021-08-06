use serde::Deserialize;
use tokio;

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
    let response = reqwest::get("http://localhost:5000/squareme?num=3")
        .await
        .unwrap()
        .json::<Response>()
        .await
        .unwrap();
    dbg!(response);
}
