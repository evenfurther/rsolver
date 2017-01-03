use std::collections::HashMap;
use super::ProjectId;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct StudentId(pub usize);

#[derive(Debug)]
pub struct Student {
    pub id: StudentId,
    pub name: String,
    pub rankings: Vec<ProjectId>,
    pub bonuses: HashMap<ProjectId, i32>,
}
