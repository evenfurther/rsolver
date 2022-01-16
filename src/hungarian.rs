#![allow(clippy::cast_possible_wrap)]

use crate::model::{Assignments, ProjectId, StudentId};
use crate::{get_config, Config};
use anyhow::{bail, Context, Error};
use pathfinding::prelude::*;
use std::collections::hash_map::HashMap;
use std::isize;
use std::iter;
use tracing::{debug, info, instrument, trace};

pub struct Hungarian<'a> {
    assignments: &'a mut Assignments,
    weights: Matrix<i64>,
}

impl<'a> Hungarian<'a> {
    pub fn new(assignments: &'a mut Assignments, config: &Config) -> Result<Hungarian<'a>, Error> {
        let rank_mult = get_config(config, "hungarian", "rank_mult")
            .unwrap_or_else(|| "3".to_owned())
            .parse::<i64>()
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

    pub fn assign(&mut self) -> Result<(), Error> {
        // Check that we have enough open positions for all our students.
        self.assignments.check_number_of_seats(false)?;

        // Proceed.
        self.do_assignments()
    }

    /// Compute the weights indexed by student then by project (less is better).
    fn compute_weights(a: &Assignments, rank_mult: i64, rank_pow: u32) -> Matrix<i64> {
        let slen = a.all_students().len() as i64;
        let mut seats = Vec::new();
        let mut seats_for = HashMap::new();
        for p in a.all_projects() {
            let n = a.max_students(p) * a.max_occurrences(p);
            seats_for.insert(
                p,
                (seats.len() as u32..seats.len() as u32 + n).collect::<Vec<_>>(),
            );
            seats.extend(iter::repeat(p).take(n as usize));
        }
        let large = i64::MAX / (1 + slen);
        let unregistered = large / (1 + slen);
        let mut weights = Matrix::new(a.all_students().len(), a.all_projects().len(), unregistered);
        for s in a.all_students() {
            for p in a.all_projects() {
                if let Some(rank) = a.rank_of(s, p) {
                    weights[(s.0, p.0)] = if a.is_pinned_and_has_chosen(s, p) {
                        -large
                    } else {
                        (rank as i64 * rank_mult).pow(rank_pow) - a.bonus(s, p).unwrap_or(0)
                    };
                }
            }
        }
        weights
    }

    /// Return the weight for a student and a project.
    fn weight_of(&self, StudentId(student): StudentId, ProjectId(project): ProjectId) -> i64 {
        self.weights[(student, project)]
    }

    /// Return the some of weights of students registered on a project.
    fn total_weight_for(&self, project: ProjectId) -> i64 {
        self.assignments
            .students_for(project)
            .iter()
            .map(|&s| self.weight_of(s, project))
            .sum::<i64>()
    }

    /// Assign every student to a project. There must be enough seats for every
    /// student or this function will panic.
    fn hungarian_algorithm(&mut self) {
        let slen = self.assignments.all_students().len();
        let mut seats = Vec::new();
        let mut seats_for = HashMap::new();
        for p in self.assignments.all_projects() {
            let n = self.assignments.max_students(p) * self.assignments.max_occurrences(p);
            seats_for.insert(
                p,
                (seats.len() as u32..seats.len() as u32 + n).collect::<Vec<_>>(),
            );
            seats.extend(iter::repeat(p).take(n as usize));
        }
        let large = i64::MAX / (1 + slen as i64);
        let mut prefs = Matrix::new(slen, seats.len(), large);
        for s in self.assignments.all_students() {
            for p in self.assignments.all_projects() {
                if !self.assignments.is_cancelled(p) {
                    let score = self.weight_of(s, p);
                    for n in &seats_for[&p] {
                        prefs[(s.0, *n as usize)] = score;
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
    #[instrument(skip_all)]
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
            if missing > unassigned.len() as u32 {
                debug!(
                    unassigned_students = %unassigned.len(),
                    necessary_students = %missing,
                    "Not enough students to complete more incomplete projects",
                );
                return;
            }
            for _ in 0..missing {
                let s = unassigned.pop().unwrap();
                trace!(
                    project = %self.assignments.project(p).name,
                    student = %self.assignments.student(s).name,
                    "Assigning to incomplete project",
                );
                self.assignments.assign_to(s, p);
            }
        }
    }

    /// Complete non-full projects with unassigned students. It will
    /// never make an acceptable project unacceptable.
    #[instrument(skip_all)]
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
                        -(self.assignments.open_spots_for(p).last().copied().unwrap() as isize),
                        self.total_weight_for(p),
                    )
                })
            {
                trace!(
                    project = %self.assignments.project(p).name,
                    student = %self.assignments.student(s).name,
                    lazy_students = %self.assignments.lazy_students_count_for(p),
                    max_open_spots = %self.assignments.open_spots_for(p).last().unwrap(),
                    "Assigning student to non-full project",
                );
                self.assignments.assign_to(s, p);
            } else {
                break;
            }
        }
    }

    /// Open new projects as needed to assign unassigned students. However,
    /// some opened projects might be unacceptable as-is.
    #[instrument(skip_all)]
    fn open_new_projects_as_needed(&mut self) {
        let mut unassigned = self.assignments.unassigned_students();
        let mut new_occurrences = false;
        while !unassigned.is_empty() {
            if let Some(p) = self
                .assignments
                .filter_projects(|p| {
                    !self.assignments.is_cancelled(p)
                        && (!self.assignments.is_open(p)
                            || (new_occurrences
                                && self.assignments.current_occurrences(p)
                                    < self.assignments.max_occurrences(p)))
                })
                .into_iter()
                .filter(|&p| self.assignments.min_students(p) <= unassigned.len() as u32)
                .min_by_key(|&p| self.assignments.project(p).min_students)
            {
                trace!(
                    project = %self.assignments.project(p).name,
                    min_students = %self.assignments.project(p).min_students,
                    "Opening new {} project",
                    if new_occurrences {
                        "occurrence of project"
                    } else {
                        "project"
                    }
                );
                for _ in 0..unassigned
                    .len()
                    .min(self.assignments.project(p).min_students as usize)
                {
                    self.assignments.assign_to(unassigned.pop().unwrap(), p);
                }
            } else {
                {
                    if new_occurrences {
                        debug!(
                            unassigned_students = %unassigned.len(),
                            "Cannot find new project to open for unassigned students"
                        );
                        break;
                    }
                    debug!("Allowing opening of new occurrences");
                    new_occurrences = true;
                }
            }
        }
    }

    /// If it exists, find one of the best unacceptable project occurrence
    /// to cancel. Or even an acceptable one if `including_acceptable` is true.
    #[instrument(skip_all)]
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
                    .copied()
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

    /// Compute the assigments, without checking that we have enough for all students.
    /// We just need to have enough for non-lazy students at this stage.
    #[instrument(skip_all)]
    fn do_assignments(&mut self) -> Result<(), Error> {
        // Check that we have enough projects for our non-lazy students.
        self.assignments.check_number_of_seats(true)?;

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
                project = %self.assignments.project(to_cancel),
                remaining_occurrences = %self.assignments.max_occurrences(to_cancel) - 1,
                "Cancelling project occurrence"
            );
            self.assignments.clear_all_assignments();
            self.assignments.cancel_occurrence(to_cancel);
            return self.do_assignments();
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
        // occurrence having only lazy students to force another larger project to open,
        if !self.assignments.unassigned_students().is_empty() {
            if let Some(to_cancel) = self.find_occurrence_to_cancel(true) {
                info!(
                    project = %self.assignments.project(to_cancel),
                    lazy_students = %self.assignments.lazy_students_count_for(to_cancel),
                    total_students = %self.assignments.students_for(to_cancel).len(),
                    remaining_occurrences = %self.assignments.max_occurrences(to_cancel) - 1,
                    "Cancelling project occurrence with too many lazy students"
                );
                self.assignments.clear_all_assignments();
                self.assignments.cancel_occurrence(to_cancel);
                return self.do_assignments();
            }
            bail!(
                "unable to assign a project to {n} students",
                n = self.assignments.unassigned_students().len()
            );
        }

        Ok(())
    }
}
