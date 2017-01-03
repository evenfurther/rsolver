extern crate ini;
#[macro_use]
extern crate mysql;
extern crate rand;

use algo::*;
use ini::Ini;
use loaders::*;
use types::*;

mod algo;
mod loaders;
mod types;

fn main() {
    let conf = Ini::load_from_file("rsolver.ini").expect("cannot load configuration file");
    let solver = conf.section(Some("solver".to_string())).expect("cannot find solver section");
    let loader = match solver.get("loader").unwrap_or(&"mysql".to_string()).as_str() {
        "mysql" => MysqlLoader {},
        other => panic!("unknown loader {}", other),
    };
    let (students, projects) = loader.load(&conf).unwrap();
    let mut assignments = Assignments::new(students, projects);
    assign(&mut assignments);
}
