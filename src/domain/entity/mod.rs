pub mod iam;

use chrono::{DateTime, Utc};
use uuid::Uuid;

pub trait Entity {
    fn ident(&self) -> Uuid;
    fn version(&self) -> u32;
    fn created(&self) -> DateTime<Utc>;
    fn updated(&self) -> Option<DateTime<Utc>>;
}

#[derive(Debug, Clone)]
pub struct EntityCtl<State> {
    id: Uuid,
    created: DateTime<Utc>,
    updated: Option<DateTime<Utc>>,
    version: u32,
    state: State,
}

/// Data used to restore a entity
#[derive(Debug, Clone)]
pub struct EntityData {
    pub(in crate::domain) id: Uuid,
    pub(in crate::domain) created: DateTime<Utc>,
    pub(in crate::domain) updated: Option<DateTime<Utc>>,
    pub(in crate::domain) version: u32,
}

impl<State> Entity for EntityCtl<State> {
    fn ident(&self) -> Uuid {
        self.id
    }

    fn version(&self) -> u32 {
        self.version
    }

    fn created(&self) -> DateTime<Utc> {
        self.created
    }

    fn updated(&self) -> Option<DateTime<Utc>> {
        self.updated
    }
}

impl<State> EntityCtl<State> {
    pub fn restore(ent: EntityData, state: State) -> Self {
        Self {
            state,
            id: ent.id,
            created: ent.created,
            updated: ent.updated,
            version: ent.version,
        }
    }

    pub fn new(state: State) -> Self {
        Self {
            id: Uuid::new_v4(),
            created: Utc::now(),
            updated: None,
            version: 1,
            state,
        }
    }
}

macro_rules! state_ref {
    ($prop:ident, $rtrn:ty) => {
        pub fn $prop(&self) -> &$rtrn {
            &self.state.$prop
        }
    };

    ($prop:ident, $rtrn:ty, $trans:block) => {
        pub fn $prop(&self) -> &$rtrn $trans
    };
}

pub(self) use state_ref;
