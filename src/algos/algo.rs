use ini::Ini;
use types::Assignments;

pub trait Algo {
    fn assign(&self, conf: &Ini, a: &mut Assignments);
}
