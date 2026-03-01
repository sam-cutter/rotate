use good_lp::{Expression, ProblemVariables, SolverModel, Variable, constraint, scip, variable};
use std::collections::HashMap;

use crate::{
    hour::{Hour, HourId},
    person::{Person, PersonId},
    role::RoleId,
};

mod hour;
mod person;
mod role;

fn main() {
    // 1 week worth of hours
    let hours: Vec<Hour> = Vec::new();
    let people: Vec<Person> = Vec::new();
    let roles: Vec<RoleId> = Vec::new();

    let mut variables = ProblemVariables::new();

    let mut assigned: HashMap<(HourId, PersonId), Variable> = HashMap::new();

    for hour in &hours {
        for person in &people {
            assigned.insert((hour.id(), person.id()), variables.add(variable().binary()));
        }
    }

    // TODO: add proper objective function
    let mut model = variables.minimise(Expression::default()).using(scip);

    // Ensures that people are only assigned to hours they are available for.
    for hour in &hours {
        for person in &people {
            model.add_constraint(constraint!(
                assigned[&(hour.id(), person.id())]
                    <= match person.available(hour.id()) {
                        true => 1,
                        false => 0,
                    }
            ));
        }
    }

    // Ensures that there are sufficient workers of each role for each shift.
    for hour in &hours {
        for role in &roles {
            let coverage = people.iter().fold(Expression::default(), |lhs, person| {
                if person.role() == *role {
                    lhs + assigned[&(hour.id(), person.id())]
                } else {
                    lhs
                }
            });

            model.add_constraint(constraint!(coverage >= hour.minimum_workers(*role) as i32));
        }
    }

    let mut persons_hours: HashMap<PersonId, u32> = HashMap::new();
}
