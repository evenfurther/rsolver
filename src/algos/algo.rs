use errors::*;
use types::Assignments;

pub trait Algo {
    fn assign(&mut self) -> Result<()>;
    fn get_assignments(&self) -> &Assignments;
}
