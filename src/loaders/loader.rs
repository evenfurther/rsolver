use errors::*;
use types::*;
use Config;

pub trait Loader {
    fn load(&mut self) -> Result<(Vec<Student>, Vec<Project>)>;
    fn save(&self, assignments: &Assignments) -> Result<()>;
}
