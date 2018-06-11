use errors::*;
use types::*;

pub trait Loader {
    fn load(&mut self) -> Result<(Vec<Student>, Vec<Project>)>;
    fn save(&self, assignments: &Assignments) -> Result<()>;
}
