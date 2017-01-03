use rand::{thread_rng, Rng};
use types::*;

fn first_non_cancelled_choice(a: &mut Assignments) {
    for student in a.unassigned_students() {
        for project in a.rankings(student).clone() {
            if !a.is_cancelled(project) {
                a.assign_to(student, project);
                break;
            }
        }
    }
}

fn solve_overflow_to_rank(a: &mut Assignments, rank: usize, rng: &mut Box<Rng>) -> bool {
    let overflowing_projects = a.filter_projects(|p| a.is_over_capacity(p));
    if overflowing_projects.is_empty() {
        return false;
    }
    println!("Overflowing projects at rank {}: {}",
             rank,
             overflowing_projects.len());
    for p in overflowing_projects.clone() {
        println!("  - {}", a.project(p).name);
    }
    let mut overflowing_students = overflowing_projects.into_iter()
        .flat_map(|p| a.students_for(p))
        .filter(|&s| !a.is_currently_pinned(*s))
        .cloned()
        .collect::<Vec<_>>();
    println!("Potential students to move: {}", overflowing_students.len());
    rng.shuffle(&mut overflowing_students);
    for student in overflowing_students {
        if let Some(project) = a.project_for(student) {
            if a.is_over_capacity(project) {
                if let Some(project) = a.project_at_rank(student, rank) {
                    if !a.is_cancelled(project) && !a.is_at_capacity(project) {
                        a.unassign(student);
                        a.assign_to(student, project);
                    }
                }
            }
        }
    }
    true
}

fn complete_incomplete_projects(a: &mut Assignments, rng: &mut Box<Rng>) {
    let mut projects = a.filter_projects(|p| a.is_under_capacity(p));
    projects.sort_by_key(|&p| (a.missing(p), -(a.size(p) as i32)));
    let mut students = a.unassigned_students();
    rng.shuffle(&mut students);
    let mut students = students.into_iter();
    for project in projects {
        while a.is_under_capacity(project) {
            if let Some(student) = students.next() {
                a.assign_to(student, project);
            } else {
                return;
            }
        }
    }
}

pub fn assign(a: &mut Assignments) {
    let mut rng: Box<Rng> = Box::new(thread_rng());
    first_non_cancelled_choice(a);
    for rank in 1..a.projects.len() {
        if !solve_overflow_to_rank(a, rank, &mut rng) {
            println!("Everyone has been assigned up to rank {}", rank);
            break;
        }
    }
    complete_incomplete_projects(a, &mut rng);
}
