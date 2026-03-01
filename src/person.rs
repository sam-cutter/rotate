use std::collections::{HashSet};

struct Person{
    max_hours_weekly : u32, min_hours_weekly : u32, pay : f32, availability : HashSet<u32>
}
impl Person{
    fn new(max_hours_weekly : u32, min_hours_weekly : u32, pay : f32, availability : HashSet<u32>) -> Self{
        Person{max_hours_weekly, min_hours_weekly, pay, availability }
    }
}
