use super::Algo;
use failure::Error;
use pathfinding::prelude::*;
use std::collections::hash_map::HashMap;
use std::isize;
use std::iter;
use types::*;

pub struct Hungarian<'a> {
    assignments: &'a mut Assignments,
    weights: Matrix<isize>,
}

impl<'a> Hungarian<'a> {
    pub fn new(assignments: &'a mut Assignments) -> Hungarian<'a> {
        let weights = Self::compute_weights(assignments);
        Hungarian {
            assignments,
            weights,
        }
    }

    /// Compute the weights indexed by student then by project (less is better).
    fn compute_weights(a: &Assignments) -> Matrix<isize> {
        let slen = a.students.len() as isize;
        let mut seats = Vec::new();
        let mut seats_for = HashMap::new();
        for p in &a.projects {
            let n = p.max_students * a.max_occurrences(p.id);
            seats_for.insert(p.id, (seats.len()..seats.len() + n).collect::<Vec<_>>());
            seats.extend(iter::repeat(p.id).take(n));
        }
        let large = isize::MAX / (1 + slen);
        let unregistered = large / (1 + slen);
        let mut weights = Matrix::new(a.students.len(), a.projects.len(), unregistered);
        for s in &a.students {
            for p in &a.projects {
                if let Some(rank) = a.rank_of(s.id, p.id) {
                    weights[&(s.id.0, p.id.0)] = if rank == 0 && a.is_pinned_for(s.id, p.id) {
                        -large
                    } else {
                        (rank as isize * 3).pow(4) - a.bonus(s.id, p.id).unwrap_or(0)
                    };
                }
            }
        }
        weights
    }

    /// Return the weight for a student and a project.
    fn weight_of(&self, StudentId(student): StudentId, ProjectId(project): ProjectId) -> isize {
        self.weights[&(student, project)]
    }

    /// Return the some of weights of students registered on a project.
    fn total_weight_for(&self, project: ProjectId) -> isize {
        self.assignments
            .students_for(project)
            .into_iter()
            .map(|&s| self.weight_of(s, project))
            .sum::<isize>()
    }

    /// Check that there are enough seats for all students.
    fn check_number_of_seats(&self) -> Result<(), Error> {
        let seats = self
            .assignments
            .all_projects()
            .into_iter()
            .map(|p| self.assignments.max_capacity(p))
            .sum::<usize>();
        ensure!(
            seats >= self.assignments.students.len(),
            "insufficient number of open projects, can host {} students out of {}",
            seats,
            self.assignments.students.len()
        );
        Ok(())
    }

    /// Assign every student to a project. There must be enough seats for every
    /// student or this function will panic.
    fn hungarian_algorithm(&mut self) {
        let slen = self.assignments.students.len() as isize;
        let mut seats = Vec::new();
        let mut seats_for = HashMap::new();
        for p in &self.assignments.projects {
            let n = p.max_students * self.assignments.max_occurrences(p.id);
            seats_for.insert(p.id, (seats.len()..seats.len() + n).collect::<Vec<_>>());
            seats.extend(iter::repeat(p.id).take(n));
        }
        let large = isize::MAX / (1 + slen);
        let mut prefs = Matrix::new(self.assignments.students.len(), seats.len(), large);
        for s in &self.assignments.students {
            for p in &self.assignments.projects {
                if !self.assignments.is_cancelled(p.id) {
                    let score = self.weight_of(s.id, p.id);
                    for n in &seats_for[&p.id] {
                        prefs[&(s.id.0, *n)] = score;
                    }
                }
            }
        }
        let (_, results) = kuhn_munkres_min(&prefs);
        for (s, seat) in results.into_iter().enumerate() {
            self.assignments.assign_to(StudentId(s), seats[seat]);
        }
    }

    /// Unassign all students who have no ranking from their assigned
    /// project.
    fn unassign_non_voting_students(&mut self) {
        for s in self.assignments.all_students() {
            let p = self.assignments.project_for(s).unwrap();
            if self.assignments.rank_of(s, p).is_none() {
                self.assignments.unassign(s);
            }
        }
    }

    /// Complete incomplete projects with unassigned students.
    fn complete_incomplete_projects(&mut self) {
        let mut unassigned = self.assignments.unassigned_students();
        let mut incomplete = self
            .assignments
            .filter_projects(|p| self.assignments.is_open(p) && !self.assignments.is_acceptable(p));
        incomplete.sort_by_key(|&p| {
            (
                self.assignments.open_spots_for(p)[0],
                self.total_weight_for(p),
            )
        });
        for p in incomplete {
            for _ in 0..self.assignments.open_spots_for(p)[0].min(unassigned.len()) {
                let s = unassigned.pop().unwrap();
                debug!(
                    "Assigning {} to incomplete project {}",
                    self.assignments.student(s).name,
                    self.assignments.project(p).name
                );
                self.assignments.assign_to(s, p);
            }
        }
    }

    /// Complete non-full projects with unassigned students. It will
    /// never make an acceptable project unacceptable.
    fn complete_non_full_projects(&mut self) {
        for s in self.assignments.unassigned_students() {
            if let Some(p) = self
                .assignments
                .filter_projects(|p| {
                    self.assignments.is_open(p) && self
                        .assignments
                        .is_acceptable_for(p, self.assignments.size(p) + 1)
                })
                .into_iter()
                .min_by_key(|&p| {
                    (
                        self.assignments.open_spots_for(p)[0],
                        self.total_weight_for(p),
                    )
                }) {
                debug!(
                    "Assigning {} to non-full project {}",
                    self.assignments.student(s).name,
                    self.assignments.project(p).name
                );
                self.assignments.assign_to(s, p);
            } else {
                break;
            }
        }
    }

    /// Open new projects as needed to assign unassigned students. However,
    /// some opened projects might be unacceptable as-is.
    fn open_new_projects_as_needed(&mut self) {
        let mut unassigned = self.assignments.unassigned_students();
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
    }

    /// If it exists, find one of the best unacceptable project occurrence
    /// to cancel.
    fn find_occurrence_to_cancel(&self) -> Option<ProjectId> {
        self.assignments
            .filter_projects(|p| {
                !self.assignments.is_cancelled(p)
                    && self.assignments.is_open(p)
                    && !self.assignments.is_acceptable(p)
            })
            .into_iter()
            .max_by_key(|&p| {
                let students = self.assignments.students_for(p);
                let pinned = students
                    .iter()
                    .filter(|&s| self.assignments.is_pinned_and_has_chosen(*s, p))
                    .count() as isize;
                let weight = self.total_weight_for(p);
                let missing = self.assignments.open_spots_for(p)[0];
                (
                    self.assignments.max_occurrences(p),
                    -pinned,
                    missing,
                    weight,
                )
            })
    }
}

impl<'a> Algo for Hungarian<'a> {
    fn assign(&mut self) -> Result<(), Error> {
        // Check that we have enough open positions for all our students.
        self.check_number_of_seats()?;

        // Run the Hungarian algorithm to assign every students to the best
        // possible project (school-wise).
        self.hungarian_algorithm();

        // Remove non-voting students for now as they will be used to
        // adjust project attendance.
        self.unassign_non_voting_students();

        // As long as we have incomplete non-empty projects, complete them with unassigned
        // students, starting with the less-incomplete projects and with the smallest
        // rank sum.
        self.complete_incomplete_projects();

        // As long as we have non-full projects, complete them with unassigned students
        // starting with projects lacking the smallest number of students and with the
        // smaller rank sum.
        self.complete_non_full_projects();

        // If we still have unassigned students, open a new project, preferring projects
        // with the smallest required number of students.
        self.open_new_projects_as_needed();

        // If we have projects which are not satisfied, remove one occurrence
        // (preferably in projects with many occurrences) and start again.
        // The number of pinned students also decreases the probability of removing
        // the project.
        if let Some(to_cancel) = self.find_occurrence_to_cancel() {
            self.assignments.clear_all_assignments();
            self.assignments.cancel_occurrence(to_cancel);
            info!(
                "Cancelling occurrence of project {}, remaining occurrences: {}",
                self.assignments.project(to_cancel).name,
                self.assignments.max_occurrences(to_cancel),
            );
            return self.assign();
        }
        Ok(())
    }

    fn get_assignments(&self) -> &Assignments {
        self.assignments
    }
}
