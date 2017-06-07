use Config;
use errors::*;
use types::*;

pub trait Loader {
    fn load(&self, config: &Config) -> Result<(Vec<Student>, Vec<Project>)>;
}
