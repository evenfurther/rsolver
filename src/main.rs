#[macro_use]
extern crate error_chain;
extern crate ini;
#[macro_use]
extern crate mysql;
extern crate rand;

use algo::*;
use errors::*;
use ini::Ini;
use loaders::*;
use stats::*;
use std::io::Write;
use types::*;

mod algo;
mod loaders;
mod stats;
mod types;

mod errors {
    error_chain!{}
}

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

fn load() -> Result<Assignments> {
    let conf = Ini::load_from_file("rsolver.ini").expect("cannot load configuration file");
    let solver = conf.section(Some("solver".to_string())).expect("cannot find solver section");
    let loader = match solver.get("loader").unwrap_or(&"mysql".to_string()).as_str() {
        "mysql" => MysqlLoader {},
        other => bail!("unknown loader: {}", other),
    };
    let (students, projects) = loader.load(&conf)?;
    Ok(Assignments::new(students, projects))
}

fn main() {
    match load() {
        Ok(mut assignments) => {
            assign(&mut assignments);
            display_stats(&assignments);
        }
        Err(e) => {
            let _ = writeln!(&mut std::io::stderr(), "Error: {:#?}", e);
            std::process::exit(1);
        }
    }
}
