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

fn solve_overflow_to_rank(a: &mut Assignments, r: usize, rng: &mut Rng) -> bool {
    let overflowing_projects = a.filter_projects(|p| a.is_over_capacity(p));
    let overflowing_students =
        overflowing_projects.into_iter().flat_map(|p| a.students_for(p)).collect::<Vec<_>>();
    rng.shuffle(&mut overflowing_students);
    true
}

pub fn assign(a: &mut Assignments) {
    let mut rng = thread_rng();
    first_non_cancelled_choice(a);
    for rank in 1..a.projects.len() {
        if !solve_overflow_to_rank(a, rank, &mut rng) {
            break;
        }
    }
}
