extern crate ini;
#[macro_use]
extern crate mysql;

use ini::Ini;
use loaders::*;
use types::*;

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
    for s in students {
        println!("{:#?}", s);
    }
    for p in projects {
        println!("Project {:?}: {}", p.id, p.name);
    }
}
