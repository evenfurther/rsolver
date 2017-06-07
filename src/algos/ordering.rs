use Config;
use errors::*;
use rand::{thread_rng, Rng};
use super::Algo;
use types::*;

pub struct Ordering<'a> {
    config: &'a Config,
    assignments: Assignments,
    rng: Box<Rng>,
}

impl<'a> Ordering<'a> {
    pub fn new(config: &'a Config, assignments: Assignments) -> Ordering<'a> {
        Ordering {
            config: config,
            assignments: assignments,
            rng: Box::new(thread_rng()),
        }
    }

    fn first_non_cancelled_choice(&mut self) {
        for student in self.assignments.unassigned_students() {
            for project in self.assignments.rankings(student).clone() {
                if !self.assignments.is_cancelled(project) {
                    self.assignments.assign_to(student, project);
                    break;
                }
            }
        }
    }

    fn solve_overflow_to_rank(&mut self, rank: usize) -> bool {
        let overflowing_projects = self.assignments
            .filter_projects(|p| self.assignments.is_over_capacity(p));
        if overflowing_projects.is_empty() {
            return false;
        }
        if self.config.verbose {
            println!("Overflowing projects at rank {}: {}",
                     rank,
                     overflowing_projects.len());
            for p in overflowing_projects.clone() {
                println!("  - {}", self.assignments.project(p).name);
            }
        }
        let mut overflowing_students = overflowing_projects
            .into_iter()
            .flat_map(|p| self.assignments.students_for(p))
            .filter(|&s| !self.assignments.is_currently_pinned(*s))
            .cloned()
            .collect::<Vec<_>>();
        if self.config.verbose {
            println!("Potential students to move: {}", overflowing_students.len());
        }
        self.rng.shuffle(&mut overflowing_students);
        for student in overflowing_students {
            if let Some(project) = self.assignments.project_for(student) {
                if self.assignments.is_over_capacity(project) {
                    if let Some(project) = self.assignments.project_at_rank(student, rank) {
                        if !self.assignments.is_cancelled(project) &&
                           !self.assignments.is_at_capacity(project) {
                            self.assignments.unassign(student);
                            self.assignments.assign_to(student, project);
                        }
                    }
                }
            }
        }
        true
    }

    fn complete_projects_under_capacity(&mut self) {
        let mut projects = self.assignments
            .filter_projects(|p| self.assignments.is_under_capacity(p));
        projects
            .sort_by_key(|&p| (self.assignments.missing(p), -(self.assignments.size(p) as isize)));
        let mut students = self.assignments.unassigned_students();
        self.rng.shuffle(&mut students);
        if self.config.verbose {
            println!("Completing {} projects under minimum capacity with {} unassigned students",
                     projects.len(),
                     students.len());
        }
        let mut students = students.into_iter();
        for project in projects {
            while self.assignments.is_under_capacity(project) {
                if let Some(student) = students.next() {
                    self.assignments.assign_to(student, project);
                } else {
                    return;
                }
            }
        }
    }

    fn cancel_occurrence_under_capacity(&mut self) -> bool {
        let mut projects = self.assignments
            .filter_projects(|p| self.assignments.is_under_capacity(p));
        if projects.is_empty() {
            return false;
        }
        projects.sort_by_key(|&p| -(self.assignments.missing(p) as isize));
        let project = projects[0];
        if self.config.verbose {
            println!("Cancelling under capacity project: {}",
                     self.assignments.project(project).name);
        }
        self.assignments.clear_all_assignments();
        self.assignments.cancel_occurrence(project);
        true
    }
}

impl<'a> Algo for Ordering<'a> {
    fn assign(&mut self) -> Result<()> {
        loop {
            self.first_non_cancelled_choice();
            for rank in 1..self.assignments.projects.len() {
                if !self.solve_overflow_to_rank(rank) {
                    if self.config.verbose {
                        println!("Everyone has been assigned up to rank {}", rank);
                    }
                    break;
                }
            }
            self.complete_projects_under_capacity();

            // If there are incomplete projects, cancel the incomplete projects with the
            // most missing members and restart.
            if !self.cancel_occurrence_under_capacity() {
                break;
            }
        }

        Ok(())
    }

    fn get_assignments(&self) -> &Assignments {
        &self.assignments
    }
}