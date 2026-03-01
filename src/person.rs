use std::collections::HashSet;

use crate::hour::HourId;

pub type PersonId = u32;

pub struct Person {
    id: PersonId,
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
}
