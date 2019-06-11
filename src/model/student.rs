use super::ProjectId;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StudentId(pub usize);

#[derive(Clone, Debug)]
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

    pub fn is_lazy(&self) -> bool {
        self.rankings.is_empty()
    }
}
