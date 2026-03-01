use good_lp::{Expression, ProblemVariables, SolverModel, Variable, constraint, scip, variable};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

use crate::{
    hour::{Hour, HourId},
    person::{Person, PersonId},
    role::RoleId,
};

mod hour;
mod person;
mod role;

const MINIMUM_SHIFT_LENGTH: u32 = 4;

fn main() {
    // Must be run for one week at a time.
    // Return (hours, blacklist_employee_pairs, people, roles, person_id_to_name)
    let test_data = test_data();
    let hours: Vec<Vec<Hour>> = Vec::new();
    let blacklist_employee_pairs: HashSet<(PersonId, PersonId)> = HashSet::new();
    let hours_flat: Vec<&Hour> = hours.iter().flat_map(|day| day.iter()).collect();
    let people: Vec<Person> = Vec::new();
    let roles: Vec<RoleId> = Vec::new();

    let mut variables = ProblemVariables::new();

    let mut assigned: HashMap<(HourId, PersonId), Variable> = HashMap::new();
    let mut assigned_length_time: HashMap<(HourId, u32, PersonId), Variable> = HashMap::new();
    let mut assigned_pairs: HashMap<(PersonId, PersonId, HourId), Variable> = HashMap::new();
    // Create the assignment variables
    for hour in &hours_flat {
        for person in &people {
            assigned.insert((hour.id(), person.id()), variables.add(variable().binary()));
        }
    }

    // Create the variables which indicate if someone is assigned for a certain length of time, starting at a certain hour
    for person in &people {
        for day in &hours {
            for length in 0..=MINIMUM_SHIFT_LENGTH {
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
    // TODO

    // Enure blacklisted employees don't work together

    for hour in &hours_flat {
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
            2,
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
            11.45,
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
            2,
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
            0,
            11.45,
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
