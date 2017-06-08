#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate flexi_logger;
extern crate ini;
#[macro_use]
extern crate log;
extern crate mysql;
extern crate rand;

use algos::*;
use clap::App;
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

fn load(config: &Config, solver: &HashMap<String, String>) -> Result<Assignments> {
    let loader = match solver.get("loader").unwrap_or(&"mysql".to_owned()).as_str() {
        "mysql" => MysqlLoader {},
        other => bail!("unknown loader: {}", other),
    };
    let (students, projects) = loader.load(config)?;
    Ok(Assignments::new(students, projects))
}

pub struct Config {
    conf: Ini,
}

impl Config {
    fn load(file_name: &str) -> Result<Config> {
        Ini::load_from_file(file_name)
            .chain_err(|| "cannot load configuration file")
            .map(|conf| Config { conf: conf })
    }
}

fn main() {
    let matches = App::new("rsolver")
        .about("Automatically assign projects to students")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .args_from_usage("
          -c,--config=[FILE] 'use FILE file instead of rsolver.ini'
          -v...              'set verbosity level'")
        .get_matches();
    let level = match matches.occurrences_of("v") {
        0 => "error",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    flexi_logger::LogOptions::new()
        .init(Some(level.to_owned()))
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
    if let Err(e) = Config::load(matches.value_of("config").unwrap_or("rsolver.ini"))
           .and_then(|conf| run(&conf)) {
        let _ = writeln!(&mut std::io::stderr(), "Error: {:#?}", e);
        std::process::exit(1);
    }
}

fn run(config: &Config) -> Result<()> {
    let solver = config
        .conf
        .section(Some("solver".to_owned()))
        .ok_or("cannot find solver section")?;
    let mut assignments = load(config, solver)?;
    {
        let mut algo = match solver
                  .get("algorithm")
                  .unwrap_or(&"ordering".to_owned())
                  .as_str() {
            "ordering" => Ordering::new(config, &mut assignments),
            other => bail!("unknown algorithm: {}", other),
        };
        algo.assign()?;
    }
    display_stats(&assignments)
}
