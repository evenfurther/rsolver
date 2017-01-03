use super::*;

#[derive(Debug)]
pub struct Assignments {
    students: Vec<Student>,
    projects: Vec<Project>,
    assigned_to: Vec<Option<usize>>,
    assigned: Vec<Vec<usize>>,
}
