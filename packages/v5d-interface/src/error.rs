use std::sync::Arc;

use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tokio::io;

pub use anyhow::Error;
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum ConnectionError {
    #[snafu(transparent)]
    SerializeMsg { source: SerializeError },
    #[snafu(transparent)]
    Remote { source: RemoteError },
    #[snafu(transparent)]
    Io { source: io::Error },
    #[snafu(whatever, display("{message}"))]
    Custom {
        message: String,
    },
}

#[derive(Debug, Snafu)]
#[snafu(
    display(
        "failed to {}serialize message {} daemon",
        if *deserialize { "de" } else { "" },
        if *deserialize { "from" } else { "to" },
    ),
    visibility(pub(crate)),
)]
pub struct SerializeError {
    source: serde_json::Error,
    pub deserialize: bool,
}

#[derive(Debug, Snafu, Clone, Serialize, Deserialize)]
#[snafu(display("{message}"), visibility(pub(crate)))]
pub struct RemoteError {
    message: Arc<str>,
}

impl From<Error> for RemoteError {
    fn from(value: Error) -> Self {
        let msg = format!("{value:?}");

        RemoteError { message: msg.into() }
    }
}
