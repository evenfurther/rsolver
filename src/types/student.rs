use super::ProjectId;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct StudentId(pub usize);

#[derive(Debug)]
pub struct Student {
    pub id: StudentId,
    pub name: String,
    pub rankings: Vec<ProjectId>,
    pub bonuses: HashMap<ProjectId, isize>,
}

impl Student {
    pub fn rank_of(&self, project: ProjectId) -> Option<usize> {
        self.rankings.iter().position(|&p| p == project)
    }
}
