use bufferfish::Bufferfish;
use serde_json::Value;
use sqlx::types::ipnetwork::IpNetwork;
use std::fmt::Display;

use crate::{error::HarpError, Loggable};

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
    pub fn new(kind: impl Display, target: &impl Loggable) -> Self {
        let (ip, id) = target.identifier();

        Self {
            id,
            addr: IpNetwork::from(ip),
            kind: kind.to_string(),
            detail: None,
            created: time::OffsetDateTime::now_utc(),
        }
    }

    /// Create an action with a detail string.
    pub fn with_detail(kind: impl Display, detail: Value, target: &impl Loggable) -> Self {
        let (ip, id) = target.identifier();

        Self {
            id,
            addr: IpNetwork::from(ip),
            kind: kind.to_string(),
            detail: Some(detail),
            created: time::OffsetDateTime::now_utc(),
        }
    }

    /// Panic-free alternative to the `From<Bufferfish> for Action` impl.
    pub fn try_from_bufferfish(mut bf: Bufferfish) -> Result<Self, HarpError> {
        let id = bf.read_u32()?;
        let addr = bf
            .read_string()?
            .parse::<IpNetwork>()
            .map_err(|_| HarpError::BadIdentifier("Invalid IP Address".to_string()))?;
        let kind = bf.read_string()?;

        let detail = bf.read_string()?;
        let detail = if detail.is_empty() { None } else { Some(serde_json::from_str(&detail)?) };

        let created = time::OffsetDateTime::now_utc();

        Ok(Self { id, addr, kind, detail, created })
    }
}

impl From<Bufferfish> for Action {
    fn from(mut bf: Bufferfish) -> Self {
        let id = bf.read_u32().unwrap();
        let addr = bf.read_string().unwrap().parse::<IpNetwork>().unwrap();
        let kind = bf.read_string().unwrap();
        let detail = bf.read_string().unwrap();
        let created = time::OffsetDateTime::now_utc();

        Self {
            id,
            addr,
            kind,
            detail: if detail.is_empty() {
                None
            } else {
                Some(serde_json::from_str(&detail).unwrap())
            },
            created,
        }
    }
}

// TODO: Possibly remove this impl and just make try_from_bufferfish the only
// way to create an Action from a Bufferfish. This being able to panic is not
// ideal for a service/server that must be resilient.
impl From<Action> for Bufferfish {
    fn from(action: Action) -> Self {
        let mut bf = Bufferfish::new();
        bf.write_u32(action.id).unwrap();
        bf.write_string(&action.addr.to_string()).unwrap();
        bf.write_string(&action.kind).unwrap();
        if action.detail.is_some() {
            bf.write_string(&serde_json::to_string(&action.detail.unwrap()).unwrap()).unwrap();
        } else {
            bf.write_string("").unwrap();
        }

        bf
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let detail = if self.detail.is_some() {
            self.detail.as_ref().unwrap().to_string()
        } else {
            "None".to_string()
        };

        write!(
            f,
            "Action {{ id: {}, addr: {}, kind: {}, detail: {}, created: {} }}",
            self.id, self.addr, self.kind, detail, self.created
        )
    }
}
