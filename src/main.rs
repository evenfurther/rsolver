extern crate ini;
#[macro_use]
extern crate mysql;

use ini::Ini;
use loaders::*;
use project::Project;
use student::Student;

mod loaders;
mod project;
mod student;

#[derive(Debug)]
struct Assignments {
    students: Vec<Student>,
    projects: Vec<Project>,
    assigned_to: Vec<Option<usize>>,
    assigned: Vec<Vec<usize>>,
}

fn main() {
    let conf = Ini::load_from_file("rsolver.ini").expect("cannot load configuration file");
    let solver = conf.section(Some("solver".to_string())).expect("cannot find solver section");
    let loader = match solver.get("loader").unwrap_or(&"mysql".to_string()).as_str() {
        "mysql" => MysqlLoader {},
        other => panic!("unknown loader {}", other),
    };
    let (mut students, mut projects) = loader.load(&conf).unwrap();
    remap(&mut students, &mut projects);
}
