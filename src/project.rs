#[derive(Debug)]
pub struct Project {
    pub id: usize,
    pub name: String,
    pub min_students: usize,
    pub max_students: usize,
    pub max_occurrences: usize,
}
