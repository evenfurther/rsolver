extern crate ini;
#[macro_use]
extern crate mysql;
extern crate rand;

use algo::*;
use ini::Ini;
use loaders::*;
use stats::*;
use types::*;

mod algo;
mod loaders;
mod stats;
mod types;

fn display_stats(a: &Assignments) {
    let ranks = statistics(a);
    let cumul = ranks.iter().scan(0, |s, &r| {
        *s += r;
        Some(*s)
    });
    let total: usize = ranks.iter().sum();
    for (rank, (n, c)) in ranks.iter().zip(cumul).enumerate() {
        if *n != 0 {
            println!("  - rank {}: {} (cumulative {} - {:.2}%)",
                     rank + 1,
                     n,
                     c,
                     100.0 * c as f32 / total as f32);
        }
    }
}

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
    display_stats(&assignments);
}
