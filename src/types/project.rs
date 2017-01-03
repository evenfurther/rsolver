#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ProjectId(pub usize);

#[derive(Debug)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub min_students: usize,
    pub max_students: usize,
    pub max_occurrences: usize,
}
