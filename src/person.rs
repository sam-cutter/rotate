use std::collections::HashSet;

use crate::{hour::HourId, role::RoleId};

pub type PersonId = u32;

pub struct Person {
    id: PersonId,
    role: RoleId,
    maximum_weekly_hours: u32,
    minimum_weekly_hours: u32,
    hourly_rate: f32,
    availability: HashSet<HourId>,
}

impl Person {
    pub fn id(&self) -> PersonId {
        self.id
    }

    pub fn available(&self, hour_id: HourId) -> bool {
        self.availability.contains(&hour_id)
    }

    pub fn role(&self) -> RoleId {
        self.role
    }

    pub fn max_weekly_hours(&self) -> u32 {
        self.maximum_weekly_hours
    }

    pub fn min_weekly_hours(&self) -> u32 {
        self.minimum_weekly_hours
    }
}
