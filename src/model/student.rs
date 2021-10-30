#![allow(clippy::module_name_repetitions)]

use super::ProjectId;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StudentId(pub usize);

#[derive(Clone, Debug)]
pub struct Student {
    pub id: StudentId,
    pub first_name: String,
    pub last_name: String,
    pub name: String,
    pub rankings: Vec<ProjectId>,
    pub bonuses: HashMap<ProjectId, i64>,
}

impl Student {
    pub fn new(
        id: StudentId,
        first_name: String,
        last_name: String,
        rankings: Vec<ProjectId>,
        bonuses: HashMap<ProjectId, i64>,
    ) -> Student {
        let name = format!("{} {}", first_name, last_name);
        Student {
            id,
            first_name,
            last_name,
            name,
            rankings,
            bonuses,
        }
    }

    pub fn rank_of(&self, project: ProjectId) -> Option<usize> {
        self.rankings.iter().position(|&p| p == project)
    }

    pub fn is_lazy(&self) -> bool {
        self.rankings.is_empty()
    }
}
