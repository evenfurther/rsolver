use super::{Project, ProjectId, Student, StudentId};
use anyhow::{ensure, Error};
use std::collections::HashMap;

const PINNING_BONUS: i64 = 1000;

#[derive(Debug)]
pub struct Assignments {
    students: Vec<Student>,
    projects: Vec<Project>,
    max_occurrences: Vec<u32>,
    assigned_to: Vec<Option<ProjectId>>,
    assigned: Vec<Vec<StudentId>>,
    pinned: Vec<Vec<StudentId>>,
}

#[allow(dead_code)]
impl Assignments {
    pub fn new(students: Vec<Student>, projects: Vec<Project>) -> Assignments {
        let number_of_students = students.len();
        let number_of_projects = projects.len();
        let pinned = (0..number_of_projects)
            .map(|project_id| {
                let project = ProjectId(project_id);
                (0..number_of_students)
                    .filter_map(|student_id| {
                        students[student_id]
                            .bonuses
                            .get(&project)
                            .and_then(|bonus| {
                                (*bonus >= PINNING_BONUS).then(|| StudentId(student_id))
                            })
                    })
                    .collect()
            })
            .collect();
        let max_occurrences = projects.iter().map(|p| p.max_occurrences).collect();
        Assignments {
            students,
            projects,
            max_occurrences,
            assigned_to: vec![None; number_of_students],
            assigned: vec![Vec::new(); number_of_projects],
            pinned,
        }
    }

    pub fn student(&self, StudentId(student): StudentId) -> &Student {
        &self.students[student]
    }

    pub fn project(&self, ProjectId(project): ProjectId) -> &Project {
        &self.projects[project]
    }

    pub fn all_projects(&self) -> Vec<ProjectId> {
        (0..self.projects.len()).map(ProjectId).collect()
    }

    pub fn filter_projects<F>(&self, condition: F) -> Vec<ProjectId>
    where
        F: Fn(ProjectId) -> bool,
    {
        (0..self.projects.len())
            .map(ProjectId)
            .filter(|&p| condition(p))
            .collect()
    }

    pub fn all_students(&self) -> Vec<StudentId> {
        (0..self.students.len()).map(StudentId).collect()
    }

    pub fn filter_students<F>(&self, condition: F) -> Vec<StudentId>
    where
        F: Fn(StudentId) -> bool,
    {
        (0..self.students.len())
            .map(StudentId)
            .filter(|&s| condition(s))
            .collect()
    }

    pub fn rankings(&self, student: StudentId) -> &Vec<ProjectId> {
        &self.student(student).rankings
    }

    pub fn bonuses(&self, student: StudentId) -> &HashMap<ProjectId, i64> {
        &self.student(student).bonuses
    }

    pub fn bonus(&self, student: StudentId, project: ProjectId) -> Option<i64> {
        self.bonuses(student).get(&project).copied()
    }

    pub fn project_for(&self, StudentId(student): StudentId) -> Option<ProjectId> {
        self.assigned_to[student]
    }

    pub fn project_at_rank(&self, student: StudentId, rank: usize) -> Option<ProjectId> {
        self.rankings(student).get(rank).copied()
    }

    pub fn rank_of(&self, student: StudentId, project: ProjectId) -> Option<usize> {
        self.student(student).rank_of(project)
    }

    pub fn students_for(&self, ProjectId(project): ProjectId) -> &Vec<StudentId> {
        &self.assigned[project]
    }

    pub fn lazy_students_for(&self, project: ProjectId) -> Vec<StudentId> {
        self.students_for(project)
            .iter()
            .filter(|&&s| self.is_lazy(s))
            .copied()
            .collect()
    }

    pub fn lazy_students_count_for(&self, project: ProjectId) -> usize {
        self.students_for(project)
            .iter()
            .filter(|&&s| self.is_lazy(s))
            .count()
    }

    pub fn pinned_students_for(&self, ProjectId(project): ProjectId) -> &Vec<StudentId> {
        &self.pinned[project]
    }

    pub fn pinned_projects_for(&self, student: StudentId) -> Vec<ProjectId> {
        self.bonuses(student)
            .iter()
            .filter_map(|(p, b)| if *b >= PINNING_BONUS { Some(*p) } else { None })
            .collect()
    }

    pub fn is_pinned_for(&self, student: StudentId, project: ProjectId) -> bool {
        self.bonuses(student)
            .get(&project)
            .map_or(false, |b| *b >= PINNING_BONUS)
    }

    pub fn is_pinned_and_has_chosen(&self, student: StudentId, project: ProjectId) -> bool {
        self.is_pinned_for(student, project) && self.rank_of(student, project) == Some(0)
    }

    pub fn is_currently_pinned(&self, student: StudentId) -> bool {
        self.project_for(student)
            .map_or(false, |project| self.is_pinned_for(student, project))
    }

    pub fn is_lazy(&self, StudentId(student): StudentId) -> bool {
        self.students[student].is_lazy()
    }

    pub fn assign_to(&mut self, student: StudentId, project: ProjectId) {
        assert!(
            self.project_for(student).is_none(),
            "a project is already assigned to this student"
        );
        assert!(
            !self.is_cancelled(project),
            "cannot assign to a cancelled project"
        );
        self.assigned_to[student.0] = Some(project);
        self.assigned[project.0].push(student);
    }

    pub fn unassign_from(&mut self, student: StudentId, project: ProjectId) {
        assert_eq!(
            self.project_for(student),
            Some(project),
            "project not assigned to this student"
        );
        self.assigned_to[student.0] = None;
        let pos = self.assigned[project.0]
            .iter()
            .position(|&s| s == student)
            .expect("student not found in project");
        self.assigned[project.0].remove(pos);
    }

    pub fn unassign(&mut self, student: StudentId) {
        let project = self
            .project_for(student)
            .expect("student is not assigned to any project");
        self.unassign_from(student, project);
    }

    pub fn clear_assignments_for(&mut self, project: ProjectId) {
        for student in self.students_for(project).clone() {
            self.unassign_from(student, project);
        }
    }

    pub fn clear_all_assignments(&mut self) {
        let projects = self.projects.iter().map(|p| p.id).collect::<Vec<_>>();
        for project in projects {
            self.clear_assignments_for(project);
        }
    }

    pub fn unassigned_students(&self) -> Vec<StudentId> {
        self.assigned_to
            .iter()
            .enumerate()
            .filter_map(|(id, assignment)| {
                if assignment.is_none() {
                    Some(StudentId(id))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn cancel(&mut self, project: ProjectId) {
        assert!(!self.is_cancelled(project), "project is cancelled already");
        self.max_occurrences[project.0] = 0;
        assert!(
            !self.is_over_capacity(project),
            "cancelled project is assign to some students"
        );
    }

    pub fn cancel_occurrence(&mut self, project: ProjectId) {
        assert!(!self.is_cancelled(project), "project is cancelled already");
        self.max_occurrences[project.0] -= 1;
        assert!(
            !self.is_over_capacity(project),
            "cancelled occurrence still has to too many students assigned"
        );
    }

    pub fn is_cancelled(&self, ProjectId(project): ProjectId) -> bool {
        self.max_occurrences[project] == 0
    }

    pub fn current_occurrences(&self, project: ProjectId) -> u32 {
        let max = self.max_students(project);
        (self.students_for(project).len() as u32 + max - 1) / max
    }

    pub fn max_occurrences(&self, ProjectId(project): ProjectId) -> u32 {
        self.max_occurrences[project]
    }

    pub fn is_open(&self, project: ProjectId) -> bool {
        !self.is_cancelled(project) && !self.students_for(project).is_empty()
    }

    pub fn size(&self, project: ProjectId) -> u32 {
        self.students_for(project).len() as u32
    }

    pub fn min_students(&self, project: ProjectId) -> u32 {
        self.project(project).min_students
    }

    pub fn max_students(&self, project: ProjectId) -> u32 {
        self.project(project).max_students
    }

    pub fn max_capacity(&self, project: ProjectId) -> u32 {
        self.max_students(project) * self.max_occurrences(project)
    }

    pub fn is_at_capacity(&self, project: ProjectId) -> bool {
        self.size(project) == self.max_capacity(project)
    }

    pub fn is_over_capacity(&self, project: ProjectId) -> bool {
        self.size(project) > self.max_capacity(project)
    }

    pub fn is_under_capacity(&self, project: ProjectId) -> bool {
        self.is_open(project) && self.size(project) < self.project(project).min_students
    }

    pub fn missing(&self, project: ProjectId) -> u32 {
        self.min_students(project) - self.size(project)
    }

    pub fn is_acceptable_for(&self, project: ProjectId, n: u32) -> bool {
        assert!(
            !self.is_cancelled(project),
            "a cancelled project cannot be acceptable"
        );
        self.project(project)
            .acceptable(self.max_occurrences(project), n)
    }

    pub fn is_acceptable(&self, project: ProjectId) -> bool {
        self.is_acceptable_for(project, self.students_for(project).len() as u32)
    }

    pub fn open_spots_for(&self, project: ProjectId) -> Vec<u32> {
        assert!(
            !self.is_cancelled(project),
            "a cancelled project cannot host anything"
        );
        let students = self.students_for(project).len() as u32;
        self.project(project)
            .can_host(self.max_occurrences(project))
            .into_iter()
            .filter_map(|n| {
                if n > students {
                    Some(n - students)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check that there are enough seats for all students.
    pub fn check_number_of_seats(&self, exclude_lazy: bool) -> Result<(), Error> {
        let seats = self
            .all_projects()
            .into_iter()
            .map(|p| self.max_capacity(p))
            .sum::<u32>();
        let students = if exclude_lazy {
            self.students.iter().filter(|s| !s.is_lazy()).count()
        } else {
            self.students.len()
        } as u32;
        ensure!(
            seats >= students,
            "insufficient number of open projects, can host {seats} {q}students out of {total}",
            q = if exclude_lazy { "non-lazy " } else { "" },
            total = self.students.len()
        );
        Ok(())
    }

    /// Unassign all students who have no ranking from their assigned
    /// project.
    pub fn unassign_non_voting_students(&mut self) {
        for s in self.all_students() {
            let p = self.project_for(s).unwrap();
            if self.rank_of(s, p).is_none() {
                self.unassign(s);
            }
        }
    }
}
