mod vms;

use framework::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[async_std::main]
async fn main() {
    FrameworkBuilder::default()
        .add_page::<Uptime>()
        .add_page::<vms::Vms>()
        .run("localhost:8080")
        .await
        .unwrap();
}

#[derive(Page, Serialize, Deserialize)]
#[page(path = "/uptime", refresh = "1s")]
pub struct Uptime {
    #[serde(serialize_with = "serialize_duration_as_string")]
    uptime: Duration,
}

fn serialize_duration_as_string<S>(
    duration: &Duration,
    s: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    format!("{}s", duration.as_secs()).serialize(s)
}

#[async_trait]
impl Constructor for Uptime {
    async fn construct(_: Request<()>) -> Result<Self> {
        match uptime_lib::get() {
            Ok(uptime) => Ok(Self { uptime }),
            Err(e) => Err(e.into()),
        }
    }
}
