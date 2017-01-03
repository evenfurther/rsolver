#[derive(Debug)]
pub struct Student {
    id: usize,
    name: String,
    rankings: Vec<usize>,
    bonuses: Vec<(usize, i32)>,
}
