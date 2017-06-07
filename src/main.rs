#[macro_use]
extern crate error_chain;
extern crate ini;
extern crate mysql;
extern crate rand;

use algos::*;
use errors::*;
use ini::Ini;
use loaders::*;
use stats::*;
use std::collections::HashMap;
use std::io::Write;
use types::*;

mod algos;
mod loaders;
mod stats;
mod types;

mod errors {
    error_chain!{}
}

fn display_stats(a: &Assignments) -> Result<()> {
    let ranks = statistics(a);
    let cumul = ranks
        .iter()
        .scan(0, |s, &r| {
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
    Ok(())
}

fn load(conf: &Ini, solver: &HashMap<String, String>) -> Result<Assignments> {
    let loader = match solver
              .get("loader")
              .unwrap_or(&"mysql".to_string())
              .as_str() {
        "mysql" => MysqlLoader {},
        other => bail!("unknown loader: {}", other),
    };
    let (students, projects) = loader.load(conf)?;
    Ok(Assignments::new(students, projects))
}

fn main() {
    if let Err(e) = run() {
        let _ = writeln!(&mut std::io::stderr(), "Error: {:#?}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let conf = Ini::load_from_file("rsolver.ini")
        .chain_err(|| "cannot load configuration file")?;
    let solver = conf.section(Some("solver".to_string()))
        .ok_or("cannot find solver section")?;
    let mut assignments = load(&conf, solver)?;
    let algo = match solver
              .get("algorithm")
              .unwrap_or(&"ordering".to_string())
              .as_str() {
        "ordering" => Ordering {},
        other => bail!("unknown algorithm: {}", other),
    };
    algo.assign(&conf, &mut assignments)?;
    display_stats(&assignments)
}
