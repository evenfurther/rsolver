use ini::Ini;
use project::Project;
use student::Student;

pub trait Loader {
    fn load(&self, config: &Ini) -> Result<(Vec<Student>, Vec<Project>), String>;
}
