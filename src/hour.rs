use std::collections::HashMap;

use crate::role::RoleId;

pub type HourId = u32;

pub struct Hour {
    id: HourId,
    minimum_workers_per_role: HashMap<RoleId, u32>,
    minimum_average_strength: f32,
}

impl Hour {
    pub fn id(&self) -> HourId {
        self.id
    }

    pub fn minimum_workers(&self, role: RoleId) -> u32 {
        self.minimum_workers_per_role[&role]
    }

    pub fn min_avg_strength(&self) -> f32 {
        self.minimum_average_strength
    }
}
