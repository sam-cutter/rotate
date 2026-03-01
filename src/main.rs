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
    // Must be run for one week at a time.

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

    let total_wages_paid = hours.iter().fold(Expression::default(), |lhs, hour| {
        lhs + people.iter().fold(Expression::default(), |lhs, person| {
            lhs + assigned[&(hour.id(), person.id())] * person.hourly_rate()
        })
    });

    let mut model = variables.minimise(total_wages_paid).using(scip);

    // Ensures that people are only assigned to hours they are available for.
    for hour in &hours {
        for person in &people {
            if person.available(hour.id()) {
                model.add_constraint(constraint!(assigned[&(hour.id(), person.id())] <= 1));
            } else {
                model.add_constraint(constraint!(assigned[&(hour.id(), person.id())] == 0));
            }
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

    // Ensures that the person is working within their range of minimum and maximum hours for that week.
    for person in &people {
        let shifts_assigned_to_this_week = hours.iter().fold(Expression::default(), |lhs, hour| {
            lhs + assigned[&(hour.id(), person.id())]
        });

        model.add_constraint(constraint!(
            shifts_assigned_to_this_week.clone() >= person.minimum_weekly_hours() as i32
        ));

        model.add_constraint(constraint!(
            shifts_assigned_to_this_week.clone() <= person.maximum_weekly_hours() as i32
        ));
    }

    // Ensures that the minimum average strength target for each hour is met.
    for hour in &hours {
        model.add_constraint(constraint!(
            people.iter().fold(Expression::default(), |lhs, person| {
                lhs + assigned[&(hour.id(), person.id())] * person.strength()
            }) >= (hour.min_avg_strength() as i32)
                * people.iter().fold(Expression::default(), |lhs, person| {
                    lhs + assigned[&(hour.id(), person.id())]
                })
        ));
    }
}
