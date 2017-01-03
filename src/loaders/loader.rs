use errors::*;
use ini::Ini;
use types::*;

pub trait Loader {
    fn load(&self, config: &Ini) -> Result<(Vec<Student>, Vec<Project>)>;
}
