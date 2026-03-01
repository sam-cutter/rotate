use std::collections::HashMap;

use crate::role::RoleId;

pub type ShiftId = u32;

pub struct Shift {
    id: ShiftId,
    minimum_workers_per_role: HashMap<RoleId, u32>,
    minimum_average_strength: f64,
}
