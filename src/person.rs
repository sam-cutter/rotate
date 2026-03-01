use crate::hour::HourId;
use std::collections::HashSet;
pub type PersonId = u32;

pub struct Person {
    id: PersonId,
    max_hours_weekly: u32,
    min_hours_weekly: u32,
    pay: f32,
    availability: HashSet<u32>,
}

impl Person {
    pub fn new(
        id: PersonId,
        max_hours_weekly: u32,
        min_hours_weekly: u32,
        pay: f32,
        availability: HashSet<u32>,
    ) -> Self {
        Person {
            id,
            max_hours_weekly,
            min_hours_weekly,
            pay,
            availability,
        }
    }

    pub fn id(&self) -> PersonId {
        self.id
    }
    pub fn available(&self, hour_id: HourId) -> bool {
        self.availability.contains(&hour_id)
    }
}
