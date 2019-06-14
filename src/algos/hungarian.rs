use super::Algo;
use crate::model::*;
use crate::{get_config, Config};
use failure::{bail, Error, ResultExt};
use pathfinding::prelude::*;
use std::collections::hash_map::HashMap;
use std::isize;
use std::iter;

pub struct Hungarian<'a> {
    assignments: &'a mut Assignments,
    weights: Matrix<isize>,
}

impl<'a> Hungarian<'a> {
    pub fn new(assignments: &'a mut Assignments, config: &Config) -> Result<Hungarian<'a>, Error> {
        let rank_mult = get_config(config, "hungarian", "rank_mult")
            .unwrap_or_else(|| "3".to_owned())
            .parse::<isize>()
            .context("cannot parse hungarian.rank_mult configuration parameter")?;
        let rank_pow = get_config(config, "hungarian", "rank_pow")
            .unwrap_or_else(|| "4".to_owned())
            .parse::<u32>()
            .context("cannot parse hungarian.rank_pow configuration parameter")?;
        let weights = Self::compute_weights(assignments, rank_mult, rank_pow);
        Ok(Hungarian {
            assignments,
            weights,
        })
    }

    /// Compute the weights indexed by student then by project (less is better).
    fn compute_weights(a: &Assignments, rank_mult: isize, rank_pow: u32) -> Matrix<isize> {
        let slen = a.students.len() as isize;
        let mut seats = Vec::new();
        let mut seats_for = HashMap::new();
        for p in a.all_projects() {
            let n = a.max_students(p) * a.max_occurrences(p);
            seats_for.insert(p, (seats.len()..seats.len() + n).collect::<Vec<_>>());
            seats.extend(iter::repeat(p).take(n));
        }
        let large = isize::MAX / (1 + slen);
        let unregistered = large / (1 + slen);
        let mut weights = Matrix::new(a.students.len(), a.all_projects().len(), unregistered);
        for s in &a.students {
            for p in a.all_projects() {
                if let Some(rank) = a.rank_of(s.id, p) {
                    weights[&(s.id.0, p.0)] = if a.is_pinned_and_has_chosen(s.id, p) {
                        -large
                    } else {
                        (rank as isize * rank_mult).pow(rank_pow) - a.bonus(s.id, p).unwrap_or(0)
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
            .iter()
            .map(|&s| self.weight_of(s, project))
            .sum::<isize>()
    }

    /// Assign every student to a project. There must be enough seats for every
    /// student or this function will panic.
    fn hungarian_algorithm(&mut self) {
        let slen = self.assignments.students.len() as isize;
        let mut seats = Vec::new();
        let mut seats_for = HashMap::new();
        for p in self.assignments.all_projects() {
            let n = self.assignments.max_students(p) * self.assignments.max_occurrences(p);
            seats_for.insert(p, (seats.len()..seats.len() + n).collect::<Vec<_>>());
            seats.extend(iter::repeat(p).take(n));
        }
        let large = isize::MAX / (1 + slen);
        let mut prefs = Matrix::new(self.assignments.students.len(), seats.len(), large);
        for s in &self.assignments.students {
            for p in self.assignments.all_projects() {
                if !self.assignments.is_cancelled(p) {
                    let score = self.weight_of(s.id, p);
                    for n in &seats_for[&p] {
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
            if unassigned.is_empty() {
                return;
            }
            let missing = self.assignments.open_spots_for(p)[0];
            if missing > unassigned.len() {
                debug!(
                    "Not enough students ({}) to complete more incomplete projects ({} necessary)",
                    unassigned.len(),
                    missing
                );
                return;
            }
            for _ in 0..missing {
                let s = unassigned.pop().unwrap();
                trace!(
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
                    self.assignments.is_open(p)
                        && self
                            .assignments
                            .is_acceptable_for(p, self.assignments.size(p) + 1)
                })
                .into_iter()
                .min_by_key(|&p| {
                    assert_eq!(self.assignments.open_spots_for(p)[0], 1);
                    (
                        self.assignments.lazy_students_count_for(p),
                        -(self.assignments.open_spots_for(p).last().cloned().unwrap() as isize),
                        self.total_weight_for(p),
                    )
                })
            {
                trace!(
                    "Assigning {} to non-full project {} with {} lazy students and {} open spots max",
                    self.assignments.student(s).name,
                    self.assignments.project(p).name,
                    self.assignments.lazy_students_count_for(p),
                    self.assignments.open_spots_for(p).last().unwrap(),
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
            match self
                .assignments
                .filter_projects(|p| {
                    !self.assignments.is_cancelled(p) && !self.assignments.is_open(p)
                })
                .into_iter()
                .filter(|&p| self.assignments.min_students(p) <= unassigned.len())
                .min_by_key(|&p| self.assignments.project(p).min_students)
            {
                Some(p) => {
                    trace!(
                        "Opening new project {} for {} students",
                        self.assignments.project(p).name,
                        self.assignments.project(p).min_students
                    );
                    for _ in 0..unassigned
                        .len()
                        .min(self.assignments.project(p).min_students)
                    {
                        self.assignments.assign_to(unassigned.pop().unwrap(), p);
                    }
                }
                None => {
                    debug!(
                        "Cannot find new project to open for {} unassigned students",
                        unassigned.len()
                    );
                    break;
                }
            }
        }
    }

    /// If it exists, find one of the best unacceptable project occurrence
    /// to cancel. Or even an acceptable one if including_acceptable is true.
    fn find_occurrence_to_cancel(&self, including_acceptable: bool) -> Option<ProjectId> {
        self.assignments
            .filter_projects(|p| {
                self.assignments.is_open(p)
                    && (including_acceptable || !self.assignments.is_acceptable(p))
            })
            .into_iter()
            .max_by_key(|&p| {
                let students = self.assignments.students_for(p);
                let pinned = students
                    .iter()
                    .filter(|&s| self.assignments.is_pinned_and_has_chosen(*s, p))
                    .count() as isize;
                let weight = self.total_weight_for(p);
                let missing = self
                    .assignments
                    .open_spots_for(p)
                    .get(0)
                    .cloned()
                    .unwrap_or(0);
                let all_lazy = students.iter().all(|&s| self.assignments.is_lazy(s));
                (
                    all_lazy,
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
        self.assignments.check_number_of_seats()?;

        // Run the Hungarian algorithm to assign every students to the best
        // possible project (school-wise).
        self.hungarian_algorithm();

        // Remove non-voting students for now as they will be used to
        // adjust project attendance.
        self.assignments.unassign_non_voting_students();

        // As long as we have incomplete non-empty projects, complete them with unassigned
        // students, starting with the less-incomplete projects and with the smallest
        // rank sum.
        self.complete_incomplete_projects();

        // If we have projects which are not satisfied, remove one occurrence
        // (preferably in projects with many occurrences) and start again.
        // The number of pinned students also decreases the probability of removing
        // the project.
        if let Some(to_cancel) = self.find_occurrence_to_cancel(false) {
            info!(
                "Cancelling occurrence of project {}, remaining occurrences: {}",
                self.assignments.project(to_cancel).name,
                self.assignments.max_occurrences(to_cancel) - 1,
            );
            self.assignments.clear_all_assignments();
            self.assignments.cancel_occurrence(to_cancel);
            return self.assign();
        }

        // As long as we have non-full projects, complete them with unassigned students
        // starting with projects lacking the smallest number of students and with the
        // smaller rank sum.
        self.complete_non_full_projects();

        // If we still have unassigned students, open new projects, preferring projects
        // with the smallest required number of students.
        self.open_new_projects_as_needed();

        // If some students are still unassigned, try to fill up newly opened projects.
        self.complete_non_full_projects();

        // If at this stage some students still cannot be assigned, cancel the smallest
        // occurrence having only lazy students to force another larger project to open.
        if !self.assignments.unassigned_students().is_empty() {
            if let Some(to_cancel) = self.find_occurrence_to_cancel(true) {
                info!(
                "Cancelling occurrence of project {} containing {} lazy students out of {} students, remaining occurrences: {}",
                self.assignments.project(to_cancel).name,
                self.assignments.lazy_students_count_for(to_cancel),
                self.assignments.students_for(to_cancel).len(),
                self.assignments.max_occurrences(to_cancel) - 1,
            );
                self.assignments.clear_all_assignments();
                self.assignments.cancel_occurrence(to_cancel);
                return self.assign();
            } else {
                bail!(
                    "unable to assign a project to {} students",
                    self.assignments.unassigned_students().len()
                );
            }
        }

        Ok(())
    }

    fn get_assignments(&self) -> &Assignments {
        self.assignments
    }
}
