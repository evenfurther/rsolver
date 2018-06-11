use super::Algo;
use errors::*;
use pathfinding::prelude::*;
use std::collections::hash_map::HashMap;
use std::isize;
use types::*;

pub struct Hungarian<'a> {
    assignments: &'a mut Assignments,
}

impl<'a> Hungarian<'a> {
    pub fn new(assignments: &'a mut Assignments) -> Hungarian<'a> {
        Hungarian { assignments }
    }

    fn hungarian_algorithm(&mut self) {
        let slen = self.assignments.students.len() as isize;
        let mut seats = Vec::new();
        let mut seats_for = HashMap::new();
        for p in &self.assignments.projects {
            let n = p.max_students * p.max_occurrences;
            seats_for.insert(p.id, (seats.len()..seats.len() + n).collect::<Vec<_>>());
            for _ in 0..n {
                seats.push(p.id);
            }
        }
        let large = isize::MAX / (1 + slen);
        let mut prefs = Matrix::new(self.assignments.students.len(), seats.len(), large);
        for s in &self.assignments.students {
            for p in &self.assignments.projects {
                if self.assignments.is_cancelled(p.id) {
                    continue;
                }
                let mut score = large / (1 + slen);
                if let Some(rank) = self.assignments.rank_of(s.id, p.id) {
                    if rank == 0 && self.assignments.is_pinned_for(s.id, p.id) {
                        score = -large;
                    } else {
                        score = (rank as isize * 3).pow(4)
                            - self.assignments.bonus(s.id, p.id).unwrap_or(0);
                    }
                }
                for n in seats_for[&p.id].iter() {
                    prefs[&(s.id.0, *n)] = score;
                }
            }
        }
        let (_, results) = kuhn_munkres_min(&prefs);
        for (s, seat) in results.into_iter().enumerate() {
            self.assignments.assign_to(StudentId(s), seats[seat]);
        }
    }
}

impl<'a> Algo for Hungarian<'a> {
    fn assign(&mut self) -> Result<()> {
        self.hungarian_algorithm();
        // Remove non-voting students for now
        for s in 0..self.assignments.students.len() {
            let s = StudentId(s);
            let p = self.assignments.project_for(s).unwrap();
            if self.assignments.rank_of(s, p).is_none() {
                self.assignments.unassign(s);
            }
        }

        // Unassigned students.
        let mut unassigned = self.assignments.unassigned_students();

        // As long as we have incomplete non-empty projects,
        // complete them with unassigned students, starting
        // with the less-incomplete projects.
        let mut incomplete = self
            .assignments
            .filter_projects(|p| self.assignments.is_under_capacity(p));
        incomplete.sort_by_key(|&p| self.assignments.missing(p));
        for p in incomplete.into_iter() {
            for _ in 0..self.assignments.missing(p).min(unassigned.len()) {
                let s = unassigned.pop().unwrap();
                self.assignments.assign_to(s, p);
            }
        }
        // As long as we have non-full projects, complete them with unassigned students
        // starting with projects lacking the smallest number of students.
        while !unassigned.is_empty() {
            if let Some(p) = self
                .assignments
                .filter_projects(|p| {
                    self.assignments.is_open(p) && !self.assignments.is_at_capacity(p)
                })
                .into_iter()
                .min_by_key(|&p| self.assignments.open_spots_for(p)[0])
            {
                self.assignments.assign_to(unassigned.pop().unwrap(), p);
            } else {
                break;
            }
        }
        // If we still have unassigned students, open a new project, preferring projects
        // with the smallest required number of students.
        while !unassigned.is_empty() {
            let p = self
                .assignments
                .filter_projects(|p| {
                    !self.assignments.is_cancelled(p) && !self.assignments.is_open(p)
                })
                .into_iter()
                .min_by_key(|&p| self.assignments.project(p).min_students)
                .unwrap();
            for _ in 0..unassigned
                .len()
                .min(self.assignments.project(p).min_students)
            {
                self.assignments.assign_to(unassigned.pop().unwrap(), p);
            }
        }
        // If we have a project with missing students, remove it from the list and restart.
        if let Some(to_cancel) = (0..self.assignments.projects.len())
            .map(ProjectId)
            .filter(|&p| {
                let n = self.assignments.students_for(p).len();
                !self.assignments.is_cancelled(p)
                    && n > 0
                    && n < self.assignments.project(p).min_students
            })
            .max_by_key(|&p| {
                let n = self.assignments.students_for(p).len();
                self.assignments.project(p).min_students - n
            }) {
                info!("Canceling project {}", self.assignments.project(to_cancel).name);
                self.assignments.clear_all_assignments();
                self.assignments.cancel(to_cancel);
                return self.assign();
            }
        Ok(())
    }

    fn get_assignments(&self) -> &Assignments {
        self.assignments
    }
}
