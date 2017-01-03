use std::collections::HashMap;
use super::*;

#[allow(dead_code)]
const PINNING_BONUS: i32 = 1000;

#[derive(Debug)]
pub struct Assignments {
    pub students: Vec<Student>,
    pub projects: Vec<Project>,
    assigned_to: Vec<Option<ProjectId>>,
    assigned: Vec<Vec<StudentId>>,
    cancelled: Vec<bool>,
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
                    .filter_map(|student_id| if let Some(bonus) = students[student_id]
                        .bonuses
                        .get(&project) {
                        if *bonus >= PINNING_BONUS {
                            Some(StudentId(student_id))
                        } else {
                            None
                        }
                    } else {
                        None
                    })
                    .collect()
            })
            .collect();
        Assignments {
            students: students,
            projects: projects,
            assigned_to: vec![None; slen],
            assigned: vec![Vec::new(); plen],
            cancelled: vec![false; plen],
            pinned: pinned,
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
        where F: Fn(ProjectId) -> bool
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

    pub fn rankings(&self, student: StudentId) -> &Vec<ProjectId> {
        &self.student(student).rankings
    }

    pub fn bonuses(&self, student: StudentId) -> &HashMap<ProjectId, i32> {
        &self.student(student).bonuses
    }

    pub fn bonus(&self, student: StudentId, project: ProjectId) -> Option<i32> {
        self.bonuses(student).get(&project).cloned()
    }

    pub fn project_for(&self, StudentId(student): StudentId) -> Option<ProjectId> {
        self.assigned_to[student]
    }

    pub fn project_at_rank(&self, student: StudentId, rank: usize) -> Option<ProjectId> {
        self.rankings(student).get(rank).cloned()
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
        self.bonuses(student).get(&project).map_or(false, |b| *b >= PINNING_BONUS)
    }

    pub fn is_currently_pinned(&self, student: StudentId) -> bool {
        if let Some(project) = self.project_for(student) {
            self.is_pinned_for(student, project)
        } else {
            false
        }
    }

    pub fn assign_to(&mut self, student: StudentId, project: ProjectId) {
        assert!(self.project_for(student).is_none(),
                "a project is already assigned to this student");
        assert!(!self.is_cancelled(project),
                "cannot assign to a cancelled project");
        self.assigned_to[student.0] = Some(project);
        self.assigned[project.0].push(student);
    }

    pub fn unassign_from(&mut self, student: StudentId, project: ProjectId) {
        assert!(self.project_for(student) == Some(project),
                "project not assigned to this student");
        self.assigned_to[student.0] = None;
        let pos = self.assigned[project.0]
            .iter()
            .position(|&s| s == student)
            .expect("student not found in project");
        self.assigned[project.0].remove(pos);
    }

    pub fn unassign(&mut self, student: StudentId) {
        let project = self.project_for(student).expect("student is not assigned to any project");
        self.unassign_from(student, project);
    }

    pub fn clear_assignments_for(&mut self, project: ProjectId) {
        for student in self.students_for(project).clone() {
            self.unassign_from(student, project);
        }
    }

    pub fn unassigned_students(&self) -> Vec<StudentId> {
        self.assigned_to
            .iter()
            .enumerate()
            .filter_map(|(id, assignment)| if assignment.is_none() {
                Some(StudentId(id))
            } else {
                None
            })
            .collect()
    }

    pub fn cancel(&mut self, ProjectId(project): ProjectId) {
        assert!(!self.cancelled[project], "project is cancelled already");
        assert!(self.assigned[project].is_empty(),
                "cancelled project is assigned to some students");
        self.cancelled[project] = true;
    }

    pub fn is_cancelled(&self, ProjectId(project): ProjectId) -> bool {
        self.cancelled[project]
    }

    pub fn is_open(&self, project: ProjectId) -> bool {
        !self.students_for(project).is_empty()
    }

    pub fn size(&self, project: ProjectId) -> usize {
        self.students_for(project).len()
    }

    pub fn min_students(&self, project: ProjectId) -> usize {
        self.project(project).min_students
    }

    pub fn is_at_capacity(&self, project: ProjectId) -> bool {
        let p = self.project(project);
        self.size(project) == p.max_students * p.max_occurrences
    }

    pub fn is_over_capacity(&self, project: ProjectId) -> bool {
        let p = self.project(project);
        self.size(project) > p.max_students * p.max_occurrences
    }

    pub fn is_under_capacity(&self, project: ProjectId) -> bool {
        self.is_open(project) && self.size(project) < self.project(project).min_students
    }

    pub fn missing(&self, project: ProjectId) -> usize {
        self.min_students(project) - self.size(project)
    }

    pub fn open_spots_for(&self, project: ProjectId) -> Vec<usize> {
        assert!(!self.is_cancelled(project), "a cancelled project cannot host anything");
        self.project(project).can_host()
    }
}
