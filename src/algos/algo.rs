use crate::model::Assignments;

pub trait Algo {
    fn assign(&mut self) -> Result<(), anyhow::Error>;
    fn get_assignments(&self) -> &Assignments;
}
