use std::collections::HashMap;

#[derive(Debug)]
pub struct Student {
    pub id: usize,
    pub name: String,
    pub rankings: Vec<usize>,
    pub bonuses: HashMap<usize, i32>,
}
