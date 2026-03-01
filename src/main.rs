use good_lp::{
    Expression, ProblemVariables, Solution, SolverModel, Variable, constraint, scip, variable,
};

use std::collections::{HashMap, HashSet};

use crate::{
    hour::{Hour, HourId},
    person::{Person, PersonId},
    role::RoleId,
};

mod hour;
mod person;
mod role;

const MINIMUM_SHIFT_LENGTH: u32 = 2;
const MAXIMUM_SHIFT_LENGTH: u32 = 8;

fn main() {
    // Must be run for one week at a time.
    let (hours, blacklist_employee_pairs, people, roles, person_id_to_name) = test_data();

    let hours_flat: Vec<&Hour> = hours.iter().flat_map(|day| day.iter()).collect();

    let mut variables = ProblemVariables::new();
    let mut assigned: HashMap<(HourId, PersonId), Variable> = HashMap::new();
    let mut assigned_to_period: HashMap<(HourId, u32, PersonId), Variable> = HashMap::new();
    let mut assigned_pairs: HashMap<(PersonId, PersonId, HourId), Variable> = HashMap::new();

    // Create the assignment variables.
    for &hour in &hours_flat {
        for person in &people {
            assigned.insert((hour.id(), person.id()), variables.add(variable().binary()));
        }
    }

    // Create the variables which indicate if someone is assigned for a certain period.
    for person in &people {
        for day in &hours {
            for period_length in (0..=MINIMUM_SHIFT_LENGTH).chain(MAXIMUM_SHIFT_LENGTH + 1..=24) {
                for (hour_index, hour) in day.iter().enumerate() {
                    if day.len() - hour_index < period_length as usize {
                        continue;
                    }

                    assigned_to_period.insert(
                        (hour.id(), period_length, person.id()),
                        variables.add(variable().binary()),
                    );
                }
            }
        }
    }

    // Create the variables which indicate if two people are on shift together.
    for person_a in &people {
        for person_b in &people {
            for hour in &hours_flat {
                assigned_pairs.insert(
                    (person_a.id(), person_b.id(), hour.id()),
                    variables.add(variable().binary()),
                );
            }
        }
    }

    // The total wages paid for this week.
    let total_wages_paid = hours_flat.iter().fold(Expression::default(), |lhs, &hour| {
        lhs + people.iter().fold(Expression::default(), |lhs, person| {
            lhs + assigned[&(hour.id(), person.id())] * person.hourly_rate()
        })
    });

    let mut model = variables.minimise(total_wages_paid).using(scip);

    // Ensures that people are only assigned to hours they are available for.
    for &hour in &hours_flat {
        for person in &people {
            if person.available(hour.id()) {
                model.add_constraint(constraint!(assigned[&(hour.id(), person.id())] <= 1));
            } else {
                model.add_constraint(constraint!(assigned[&(hour.id(), person.id())] == 0));
            }
        }
    }

    // Ensures that there are sufficient workers of each role for each hour.
    for &hour in &hours_flat {
        for role in &roles {
            let coverage = people
                .iter()
                .filter(|&person| person.role() == *role)
                .fold(Expression::default(), |lhs, person| {
                    lhs + assigned[&(hour.id(), person.id())]
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
    for &hour in &hours_flat {
        let total_strength = people.iter().fold(Expression::default(), |lhs, person| {
            lhs + assigned[&(hour.id(), person.id())] * person.strength()
        });

        let minimum_average_strength = hour.minimum_average_strength() as i32;

        let people_working_this_hour = people.iter().fold(Expression::default(), |lhs, person| {
            lhs + assigned[&(hour.id(), person.id())]
        });

        model.add_constraint(constraint!(
            total_strength >= minimum_average_strength * people_working_this_hour
        ));
    }

    // Makes sure that the variables which indicate if someone is assigned for a certain period have the correct value.
    for person in &people {
        for day in &hours {
            for length in (1..=MINIMUM_SHIFT_LENGTH).chain(MAXIMUM_SHIFT_LENGTH + 1..=24) {
                for (start_hour_index, hour) in day.iter().enumerate() {
                    if day.len() - start_hour_index < length as usize {
                        continue;
                    }

                    // This variable indicates if someone is assigned to a certain period.
                    // This is the variable we are trying to ensure is correct.
                    let is_assigned_to_period =
                        assigned_to_period[&(hour.id(), length, person.id())];

                    // The number of hours which the person is assigned to within this period.
                    let assigned_hours_in_this_period =
                        (0..length as usize).fold(Expression::default(), |lhs, hour_offset| {
                            lhs + assigned[&(day[start_hour_index + hour_offset].id(), person.id())]
                        });

                    // Forces the variable to be 0 if it should be 0
                    model.add_constraint(constraint!(
                        is_assigned_to_period * length <= assigned_hours_in_this_period.clone()
                    ));

                    // Forces the variable to be 1 if it should be 1
                    model.add_constraint(constraint!(
                        is_assigned_to_period * length
                            >= assigned_hours_in_this_period - length + 1
                    ));
                }
            }
        }
    }

    // Ensures that no shifts are less long than the minimum shift length.
    for person in &people {
        for day in &hours {
            for length in 2..=MINIMUM_SHIFT_LENGTH {
                // All shifts of length n - 1
                let lhs = day.iter().enumerate().fold(
                    Expression::default(),
                    |lhs, (start_hour_index, hour)| {
                        if day.len() - start_hour_index < (length - 1) as usize {
                            lhs
                        } else {
                            lhs + assigned_to_period[&(hour.id(), length - 1, person.id())]
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
                            lhs + assigned_to_period[&(hour.id(), length, person.id())]
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
                            lhs + assigned_to_period[&(hour.id(), length, person.id())]
                        }
                    },
                );

                model.add_constraint(constraint!(lhs == 0));
            }
        }
    }

    // Ensures that blacklisted pairings are not created.
    for &hour in &hours_flat {
        for person_a in &people {
            for person_b in &people {
                let paired: Variable = assigned_pairs[&(person_a.id(), person_b.id(), hour.id())];

                let a_assigned: Variable = assigned[&(hour.id(), person_a.id())];
                let b_assigned: Variable = assigned[&(hour.id(), person_b.id())];

                model.add_constraint(constraint!(paired <= a_assigned));
                model.add_constraint(constraint!(paired <= b_assigned));
                model.add_constraint(constraint!(paired >= a_assigned + b_assigned - 1));

                if blacklist_employee_pairs.contains(&(person_a.id(), person_b.id()))
                    || blacklist_employee_pairs.contains(&(person_b.id(), person_a.id()))
                {
                    model.add_constraint(constraint!(paired == 0));
                }
            }
        }
    }

    let solution = model.solve().unwrap();

    for (day_index, day) in hours.iter().enumerate() {
        println!("\n\nDAY {}", day_index);

        for (hour_index, hour) in day.iter().enumerate() {
            print!("hour {}: ", hour_index);

            let assigned: Vec<String> = people
                .iter()
                .filter(|person| solution.value(assigned[&(hour.id(), person.id())]) == 1.0)
                .map(|person| person_id_to_name[&person.id()].clone())
                .collect();

            println!("{}", assigned.join(", "));
        }
    }
}

fn test_data() -> (
    Vec<Vec<Hour>>,
    HashSet<(PersonId, PersonId)>,
    Vec<Person>,
    Vec<RoleId>,
    HashMap<PersonId, String>,
) {
    // Return (hours, blacklist_employee_pairs, people, roles, person_id_to_name)
    // id 9 is monday 9am to 10am
    // id 220 is wednesday (as 2 in hunderds so 2 days after monday) from 8:00 pm to 9:00 pm
    // role 0 is only role
    let mut hours: Vec<Vec<Hour>> = vec![vec![]];
    let hour_id_to_needed_staff: HashMap<HourId, u32> = vec![
        (9, 2),
        (10, 3),
        (11, 4),
        (12, 5),
        (13, 5),
        (14, 5),
        (15, 5),
        (16, 4),
        (17, 4),
        (18, 6),
        (19, 6),
        (20, 6),
        (21, 3),
    ]
    .iter()
    .cloned()
    .collect();
    for hour_id in 9..=21 {
        hours[0].push(Hour::new(
            (hour_id) as u32,
            vec![(0, hour_id_to_needed_staff[&hour_id])]
                .iter()
                .cloned()
                .collect(),
            7.0,
        ))
    }
    // blacklist alex id 0 and shay id 1
    let blacklist_employee_pairs: HashSet<(PersonId, PersonId)> = HashSet::new();

    let person_id_to_name: HashMap<PersonId, String> = vec![
        (0, String::from("CheapM1")),
        (1, String::from("CheapM2")),
        (2, String::from("CheapM3")),
        (3, String::from("ExpM1")),
        (4, String::from("ExpM2")),
        (5, String::from("ExpM3")),
        (6, String::from("ExpNNM1")),
        (7, String::from("ExpNNM2")),
        (8, String::from("ExpNN3")),
        (9, String::from("CheapE1")),
        (10, String::from("CheapE2")),
        (11, String::from("CheapE3")),
        (12, String::from("ExpE1")),
        (13, String::from("ExpE2")),
        (14, String::from("ExpE3")),
        (15, String::from("ExpNNE1")),
        (16, String::from("ExpNNE2")),
        (17, String::from("ExpNNE3")),
    ]
    .iter()
    .cloned()
    .collect();
    let mut people: Vec<Person> = Vec::new();

    // 0 to 2 are cheap, min 2 hours, morning team
    for person_id in 0..=2 {
        people.push(Person::new(
            person_id,
            0,
            8,
            0,
            9.45,
            vec![9, 10, 11, 12, 13, 14, 15].iter().cloned().collect(),
            10.0,
        ))
    }
    // expensive, min 2 hours, morning team
    for person_id in 3..=5 {
        people.push(Person::new(
            person_id,
            0,
            8,
            2,
            11.45,
            vec![9, 10, 11, 12, 13, 14, 15].iter().cloned().collect(),
            10.0,
        ))
    }
    // expensive, min 0 hours, morning team
    for person_id in 6..=8 {
        people.push(Person::new(
            person_id,
            0,
            8,
            0,
            12.45,
            vec![9, 10, 11, 12, 13, 14, 15].iter().cloned().collect(),
            10.0,
        ))
    }
    // cheap, min 2 hours, night team
    for person_id in 9..=11 {
        people.push(Person::new(
            person_id,
            0,
            8,
            0,
            9.45,
            vec![16, 17, 18, 19, 20, 21].iter().cloned().collect(),
            10.0,
        ))
    }
    // expensive, min 2 hours, night team
    for person_id in 12..=14 {
        people.push(Person::new(
            person_id,
            0,
            8,
            2,
            11.45,
            vec![16, 17, 18, 19, 20, 21].iter().cloned().collect(),
            10.0,
        ))
    }
    // expensive, min 0 hours, morning team
    for person_id in 15..=17 {
        people.push(Person::new(
            person_id,
            0,
            8,
            2,
            12.45,
            vec![16, 17, 18, 19, 20, 21].iter().cloned().collect(),
            10.0,
        ))
    }
    let roles: Vec<RoleId> = vec![0];
    // Return (hours, blacklist_employee_pairs,  people, roles, person_id_to_name)
    (
        hours,
        blacklist_employee_pairs,
        people,
        roles,
        person_id_to_name,
    )
}

// fn test_data() -> (
//     Vec<Vec<Hour>>,
//     HashSet<(PersonId, PersonId)>,
//     Vec<Person>,
//     Vec<RoleId>,
//     HashMap<PersonId, String>,
// ) {
//     let hours = vec![vec![Hour::new(
//         0,
//         vec![(0, 1)].iter().cloned().collect(),
//         7.0,
//     )]];

//     let blacklisted_pairs = HashSet::new();

//     let people = vec![Person::new(
//         0,
//         0,
//         8,
//         0,
//         9.45,
//         vec![0].iter().cloned().collect(),
//         10.0,
//     )];

//     let roles = vec![0];

//     let mapping = vec![(0, "Shay".to_string())].iter().cloned().collect();

//     return (hours, blacklisted_pairs, people, roles, mapping);
// }
