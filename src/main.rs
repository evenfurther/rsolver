#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate ini;
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
    let loader = match solver
              .get("loader")
              .unwrap_or(&"mysql".to_string())
              .as_str() {
        "mysql" => MysqlLoader {},
        other => bail!("unknown loader: {}", other),
    };
    let (students, projects) = loader.load(config)?;
    Ok(Assignments::new(students, projects))
}

pub struct Config {
    verbose: bool,
    conf: Ini,
}

impl Config {
    fn load(file_name: &str, verbose: bool) -> Result<Config> {
        Ini::load_from_file(file_name)
            .chain_err(|| "cannot load configuration file")
            .map(|conf| {
                     Config {
                         verbose: verbose,
                         conf: conf,
                     }
                 })
    }
}

fn main() {
    let matches = App::new("rsolver")
        .about("Automatically assign projects to students")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .args_from_usage("
          -c,--config=[FILE] 'use FILE file instead of rsolver.ini'
          -v,--verbose       'be verbose'")
        .get_matches();
    if let Err(e) = Config::load(matches.value_of("config").unwrap_or("rsolver.ini"),
                                 matches.is_present("verbose"))
               .and_then(|conf| run(&conf)) {
        let _ = writeln!(&mut std::io::stderr(), "Error: {:#?}", e);
        std::process::exit(1);
    }
}

fn run(config: &Config) -> Result<()> {
    let solver = config
        .conf
        .section(Some("solver".to_string()))
        .ok_or("cannot find solver section")?;
    let mut assignments = load(config, solver)?;
    {
        let mut algo = match solver
                  .get("algorithm")
                  .unwrap_or(&"ordering".to_string())
                  .as_str() {
            "ordering" => Ordering::new(config, &mut assignments),
            other => bail!("unknown algorithm: {}", other),
        };
        algo.assign()?;
    }
    display_stats(&assignments)
}
