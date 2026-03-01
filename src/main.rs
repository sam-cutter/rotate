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

const MINIMUM_SHIFT_LENGTH: u32 = 4;
const MAXIMUM_SHIFT_LENGTH: u32 = 8;

fn main() {
    // Must be run for one week at a time.

    let hours: Vec<Vec<Hour>> = Vec::new();

    let hours_flat: Vec<&Hour> = hours.iter().flat_map(|day| day.iter()).collect();
    let people: Vec<Person> = Vec::new();
    let roles: Vec<RoleId> = Vec::new();

    let mut variables = ProblemVariables::new();

    let mut assigned: HashMap<(HourId, PersonId), Variable> = HashMap::new();
    let mut assigned_length_time: HashMap<(HourId, u32, PersonId), Variable> = HashMap::new();

    // Create the assignment variables
    for hour in &hours_flat {
        for person in &people {
            assigned.insert((hour.id(), person.id()), variables.add(variable().binary()));
        }
    }

    // Create the variables which indicate if someone is assigned for a certain length of time, starting at a certain hour
    for person in &people {
        for day in &hours {
            for length in (0..=MINIMUM_SHIFT_LENGTH).chain(MAXIMUM_SHIFT_LENGTH + 1..=24) {
                for (i, hour) in day.iter().enumerate() {
                    if day.len() - i <= length as usize {
                        continue;
                    }

                    assigned_length_time.insert(
                        (hour.id(), length, person.id()),
                        variables.add(variable().binary()),
                    );
                }
            }
        }
    }

    let total_wages_paid = hours_flat.iter().fold(Expression::default(), |lhs, hour| {
        lhs + people.iter().fold(Expression::default(), |lhs, person| {
            lhs + assigned[&(hour.id(), person.id())] * person.hourly_rate()
        })
    });

    let mut model = variables.minimise(total_wages_paid).using(scip);

    // Ensures that people are only assigned to hours they are available for.
    for hour in &hours_flat {
        for person in &people {
            if person.available(hour.id()) {
                model.add_constraint(constraint!(assigned[&(hour.id(), person.id())] <= 1));
            } else {
                model.add_constraint(constraint!(assigned[&(hour.id(), person.id())] == 0));
            }
        }
    }

    // Ensures that there are sufficient workers of each role for each shift.
    for hour in &hours_flat {
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
        let shifts_assigned_to_this_week =
            hours_flat.iter().fold(Expression::default(), |lhs, hour| {
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
    for hour in &hours_flat {
        model.add_constraint(constraint!(
            people.iter().fold(Expression::default(), |lhs, person| {
                lhs + assigned[&(hour.id(), person.id())] * person.strength()
            }) >= (hour.min_avg_strength() as i32)
                * people.iter().fold(Expression::default(), |lhs, person| {
                    lhs + assigned[&(hour.id(), person.id())]
                })
        ));
    }

    // Makes sure that the variables which indicate if someone is assigned for a certain length of time, starting at a certain hour are synced
    for person in &people {
        for day in &hours {
            for length in (0..=MINIMUM_SHIFT_LENGTH).chain(MAXIMUM_SHIFT_LENGTH + 1..=24) {
                for (start_hour_index, hour) in day.iter().enumerate() {
                    if day.len() - start_hour_index <= length as usize {
                        continue;
                    }

                    let is_assigned_length_time =
                        assigned_length_time[&(hour.id(), length, person.id())];

                    let count_assigned_shifts_in_period =
                        (0..length as usize).fold(Expression::default(), |lhs, hour_offset| {
                            lhs + assigned[&(day[start_hour_index + hour_offset].id(), person.id())]
                        });

                    // Forces the variable to be 0 if it should be 0
                    model.add_constraint(constraint!(
                        is_assigned_length_time * length <= count_assigned_shifts_in_period.clone()
                    ));

                    // Forces the variable to be 1 if it should be 1
                    model.add_constraint(constraint!(
                        is_assigned_length_time * length
                            >= count_assigned_shifts_in_period - length + 1
                    ));
                }
            }
        }
    }

    // Ensures that no shifts are less long than the minimum shift length
    for person in &people {
        for day in &hours {
            for length in 2..=MINIMUM_SHIFT_LENGTH {
                // All shifts of length n - 1
                let lhs = day.iter().enumerate().fold(
                    Expression::default(),
                    |lhs, (start_hour_index, hour)| {
                        if day.len() - start_hour_index <= (length - 1) as usize {
                            lhs
                        } else {
                            lhs + assigned_length_time[&(hour.id(), length - 1, person.id())]
                        }
                    },
                );

                // All shifts of length n
                let rhs = day.iter().enumerate().fold(
                    Expression::default(),
                    |lhs, (start_hour_index, hour)| {
                        if day.len() - start_hour_index <= length as usize {
                            lhs
                        } else {
                            lhs + assigned_length_time[&(hour.id(), length, person.id())]
                        }
                    },
                );

                // The number of shifts of length (n - 1) must be equal to the number of shifts of length n, plus 1
                model.add_constraint(constraint!(lhs == rhs + 1));
            }
        }
    }

    // Ensures that no shifts are longer than the maximum shift length.
    for person in &people {
        for day in &hours {
            for length in MAXIMUM_SHIFT_LENGTH + 1..=24 {
                let lhs = day.iter().enumerate().fold(
                    Expression::default(),
                    |lhs, (start_hour_index, hour)| {
                        if day.len() - start_hour_index <= length as usize {
                            lhs
                        } else {
                            lhs + assigned_length_time[&(hour.id(), length, person.id())]
                        }
                    },
                );

                // The number of shifts of length (n - 1) must be equal to the number of shifts of length n, plus 1
                model.add_constraint(constraint!(lhs == 0));
            }
        }
    }
}
