use std::fmt::Display;

use bufferfish::Bufferfish;
use serde_json::Value;
use sqlx::types::ipnetwork::IpNetwork;
use time::{macros::format_description, OffsetDateTime};

use crate::{error::HarpError, Loggable};

/// Represents a "kind" of action. Implementing this trait requires the `key()`
/// method, which should return a string representation of the action kind. This
/// string should be unique and, ideally, small.
///
/// The return value will be stored in the database, so consider that when
/// deciding on a key.
pub trait Kind {
    fn key(&self) -> &str;
}

/// Represents a "complete" action to be logged into the database at a later
/// time. Actions are primarily defined by their kind, which is a string
/// representation of the action that occurred. They can include optional
/// details.
#[derive(Debug)]
pub struct Action {
    pub id: u32,
    pub addr: IpNetwork,
    pub kind: String,
    pub detail: Option<Value>,
    pub created: time::OffsetDateTime,
}

impl Action {
    /// Create a basic action that has no extraneous details.
    pub fn new(kind: impl Kind, target: &impl Loggable) -> Self {
        let (ip, id) = target.identifier();

        Self {
            id,
            addr: IpNetwork::from(ip),
            kind: kind.key().to_string(),
            detail: None,
            created: time::OffsetDateTime::now_utc(),
        }
    }

    /// Create an action with a detail string.
    pub fn with_detail(kind: impl Kind, detail: Value, target: &impl Loggable) -> Self {
        let (ip, id) = target.identifier();

        Self {
            id,
            addr: IpNetwork::from(ip),
            kind: kind.key().to_string(),
            detail: Some(detail),
            created: time::OffsetDateTime::now_utc(),
        }
    }
}

impl TryFrom<Bufferfish> for Action {
    type Error = HarpError;

    fn try_from(mut value: Bufferfish) -> Result<Self, Self::Error> {
        let id = value.read_u32()?;

        let addr = value
            .read_string()?
            .parse::<IpNetwork>()
            .map_err(|_| HarpError::BadIdentifier("Invalid IP Address".to_string()))?;

        let kind = value.read_string()?;

        let detail = value.read_string()?;
        let detail = if detail.is_empty() { None } else { Some(serde_json::from_str(&detail)?) };

        let created = value.read_string()?;

        // 2023-02-24 13:01:12.558038011 +00:00:00
        let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond] [offset_hour]:[offset_minute]:[offset_second]");
        let created = OffsetDateTime::parse(&created, format)?;

        Ok(Self { id, addr, kind, detail, created })
    }
}

impl TryFrom<Action> for Bufferfish {
    type Error = HarpError;

    fn try_from(value: Action) -> Result<Self, Self::Error> {
        let mut bf = Bufferfish::new();
        bf.write_u32(value.id)?;
        bf.write_string(&value.addr.to_string())?;
        bf.write_string(&value.kind)?;

        match value.detail {
            Some(detail) => bf.write_string(&serde_json::to_string(&detail)?)?,
            None => bf.write_string("")?,
        }

        bf.write_string(&value.created.to_string())?;

        Ok(bf)
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let detail = match &self.detail {
            Some(d) => d.to_string(),
            None => "None".to_string(),
        };

        write!(
            f,
            "Action {{ id: {}, addr: {}, kind: {}, detail: {}, created: {} }}",
            self.id, self.addr, self.kind, detail, self.created
        )
    }
}
