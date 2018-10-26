use crate::types::*;
use failure::Error;

pub trait Loader {
    fn load(&mut self) -> Result<(Vec<Student>, Vec<Project>), Error>;
    fn save(&self, assignments: &Assignments) -> Result<(), Error>;
}
