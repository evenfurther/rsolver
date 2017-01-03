#[derive(Debug)]
pub struct Student {
    pub id: usize,
    pub name: String,
    pub rankings: Vec<usize>,
    pub bonuses: Vec<(usize, i32)>,
}
