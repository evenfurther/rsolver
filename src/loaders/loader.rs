use errors::*;
use types::*;
use Config;

pub trait Loader {
    fn load(&self, config: &Config) -> Result<(Vec<Student>, Vec<Project>)>;
}
