pub mod iam;

use chrono::{DateTime, Utc};
use uuid::Uuid;

pub trait Entity {
    fn ident(&self) -> Uuid;
    fn version(&self) -> u32;
    fn created(&self) -> DateTime<Utc>;
    fn updated(&self) -> Option<DateTime<Utc>>;
}

/// Data used to restore a entity
#[derive(Debug, Clone)]
pub struct EntityData {
    pub(in crate::domain) id: Uuid,
    pub(in crate::domain) created: DateTime<Utc>,
    pub(in crate::domain) updated: Option<DateTime<Utc>>,
    pub(in crate::domain) version: u32,
}

impl EntityData {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            created: Utc::now(),
            updated: None,
            version: 1,
        }
    }

    /// Update the entity data.
    ///
    /// Icrement the entity version by 1 and set the updated time as now.
    pub fn update(&mut self) {
        self.updated = Some(Utc::now());
        self.version += 1;
    }
}

macro_rules! transform_helper {
    ($state_ty:ty) => {
        pub(in crate::domain) fn restore(
            data: crate::domain::entity::EntityData,
            state: $state_ty,
        ) -> Self {
            Self { state, data }
        }

        pub(in crate::domain) fn unmount_state(
            self,
        ) -> (crate::domain::entity::EntityData, $state_ty) {
            (self.data, self.state)
        }
    };
}

macro_rules! impl_entity {
    ($entity_ty:ty) => {
        impl crate::domain::entity::Entity for $entity_ty {
            fn ident(&self) -> uuid::Uuid {
                self.data.id
            }

            fn version(&self) -> u32 {
                self.data.version
            }

            fn created(&self) -> chrono::DateTime<chrono::Utc> {
                self.data.created
            }

            fn updated(&self) -> Option<chrono::DateTime<chrono::Utc>> {
                self.data.updated
            }
        }
    };
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

pub(self) use impl_entity;
pub(self) use state_ref;
pub(self) use transform_helper;
