use good_lp::{Expression, ProblemVariables, Variable, constraint, variable};
use std::collections::HashMap;

use crate::{
    hour::{Hour, HourId},
    person::{Person, PersonId},
};

mod hour;
mod person;
mod role;

fn main() {
    let hours: Vec<Hour> = Vec::new();
    let people: Vec<Person> = Vec::new();

    let mut variables = ProblemVariables::new();

    let mut assigned: HashMap<(HourId, PersonId), Variable> = HashMap::new();

    for hour in &hours {
        for person in &people {
            assigned.insert((hour.id(), person.id()), variables.add(variable().binary()));
        }
    }
}
