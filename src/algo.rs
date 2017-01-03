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
    let mut overflowing_students =
        overflowing_projects.into_iter().flat_map(|p| a.students_for(p)).cloned().collect::<Vec<_>>();
    if overflowing_students.is_empty() {
        return false;
    }
    rng.shuffle(&mut overflowing_students);
    for student in overflowing_students {
        if let Some(project) = a.project_for(student) {
            if a.is_over_capacity(project) && !a.is_pinned_for(student, project) {
                if let Some(project) = a.project_at_rank(student, rank) {
                    if !a.is_over_capacity(project) {
                        a.unassign(student);
                        a.assign_to(student, project);
                    }
                }
            }
        }
    }
    true
}

pub fn assign(a: &mut Assignments) {
    let mut rng: Box<Rng> = Box::new(thread_rng());
    first_non_cancelled_choice(a);
    for rank in 1..a.projects.len() {
        if !solve_overflow_to_rank(a, rank, &mut rng) {
            break;
        }
    }
}
