use super::*;
use failure::Error;
use std::collections::HashMap;

const PINNING_BONUS: isize = 1000;

#[derive(Debug)]
pub struct Assignments {
    pub students: Vec<Student>,
    pub projects: Vec<Project>,
    max_occurrences: Vec<usize>,
    assigned_to: Vec<Option<ProjectId>>,
    assigned: Vec<Vec<StudentId>>,
    pinned: Vec<Vec<StudentId>>,
}

#[allow(dead_code)]
impl Assignments {
    pub fn new(students: Vec<Student>, projects: Vec<Project>) -> Assignments {
        let slen = students.len();
        let plen = projects.len();
        let pinned = (0..plen)
            .map(|project_id| {
                let project = ProjectId(project_id);
                (0..slen)
                    .filter_map(|student_id| {
                        if let Some(bonus) = students[student_id].bonuses.get(&project) {
                            if *bonus >= PINNING_BONUS {
                                Some(StudentId(student_id))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .collect();
        let max_occurrences = projects.iter().map(|p| p.max_occurrences).collect();
        Assignments {
            students,
            projects,
            max_occurrences,
            assigned_to: vec![None; slen],
            assigned: vec![Vec::new(); plen],
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
        self.filter_projects(|_| true)
    }

    pub fn filter_projects<F>(&self, condition: F) -> Vec<ProjectId>
    where
        F: Fn(ProjectId) -> bool,
    {
        (0..self.projects.len())
            .filter_map(|project| {
                let project = ProjectId(project);
                if condition(project) {
                    Some(project)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn all_students(&self) -> Vec<StudentId> {
        (0..self.students.len()).map(StudentId).collect()
    }

    pub fn rankings(&self, student: StudentId) -> &Vec<ProjectId> {
        &self.student(student).rankings
    }

    pub fn bonuses(&self, student: StudentId) -> &HashMap<ProjectId, isize> {
        &self.student(student).bonuses
    }

    pub fn bonus(&self, student: StudentId, project: ProjectId) -> Option<isize> {
        self.bonuses(student).get(&project).cloned()
    }

    pub fn project_for(&self, StudentId(student): StudentId) -> Option<ProjectId> {
        self.assigned_to[student]
    }

    pub fn project_at_rank(&self, student: StudentId, rank: usize) -> Option<ProjectId> {
        self.rankings(student).get(rank).cloned()
    }

    pub fn rank_of(&self, student: StudentId, project: ProjectId) -> Option<usize> {
        self.student(student).rank_of(project)
    }

    pub fn students_for(&self, ProjectId(project): ProjectId) -> &Vec<StudentId> {
        &self.assigned[project]
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
        if let Some(project) = self.project_for(student) {
            self.is_pinned_for(student, project)
        } else {
            false
        }
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

    pub fn max_occurrences(&self, ProjectId(project): ProjectId) -> usize {
        self.max_occurrences[project]
    }

    pub fn is_open(&self, project: ProjectId) -> bool {
        !self.is_cancelled(project) && !self.students_for(project).is_empty()
    }

    pub fn size(&self, project: ProjectId) -> usize {
        self.students_for(project).len()
    }

    pub fn min_students(&self, project: ProjectId) -> usize {
        self.project(project).min_students
    }

    pub fn max_students(&self, project: ProjectId) -> usize {
        self.project(project).max_students
    }

    pub fn max_capacity(&self, project: ProjectId) -> usize {
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

    pub fn missing(&self, project: ProjectId) -> usize {
        self.min_students(project) - self.size(project)
    }

    pub fn is_acceptable_for(&self, project: ProjectId, n: usize) -> bool {
        assert!(
            !self.is_cancelled(project),
            "a cancelled project cannot be acceptable"
        );
        self.project(project)
            .acceptable(self.max_occurrences(project), n)
    }

    pub fn is_acceptable(&self, project: ProjectId) -> bool {
        self.is_acceptable_for(project, self.students_for(project).len())
    }

    pub fn open_spots_for(&self, project: ProjectId) -> Vec<usize> {
        assert!(
            !self.is_cancelled(project),
            "a cancelled project cannot host anything"
        );
        let students = self.students_for(project).len();
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
    pub fn check_number_of_seats(&self) -> Result<(), Error> {
        let seats = self
            .all_projects()
            .into_iter()
            .map(|p| self.max_capacity(p))
            .sum::<usize>();
        ensure!(
            seats >= self.students.len(),
            "insufficient number of open projects, can host {} students out of {}",
            seats,
            self.students.len()
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
