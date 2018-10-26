use crate::types::Assignments;
use failure::Error;

pub trait Algo {
    fn assign(&mut self) -> Result<(), Error>;
    fn get_assignments(&self) -> &Assignments;
}
