use crate::types::*;

pub fn statistics(a: &Assignments) -> Vec<usize> {
    let mut ranks = vec![0; a.projects.len()];
    for project in a.filter_projects(|p| a.is_open(p)) {
        for &student in a.students_for(project) {
            if let Some(rank) = a.rank_of(student, project) {
                ranks[rank] += 1;
            }
        }
    }
    let latest = ranks.iter().rposition(|&n| n != 0).map_or(0, |n| n + 1);
    ranks.truncate(latest);
    ranks
}
