#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;
extern crate flexi_logger;
extern crate ini;
#[macro_use]
extern crate log;
#[macro_use]
extern crate mysql;
extern crate pathfinding;
extern crate rand;

use crate::algos::*;
use crate::loaders::*;
use crate::stats::*;
use crate::types::*;
use clap::App;
use failure::*;
use ini::Ini;

mod algos;
mod loaders;
mod stats;
mod types;

fn display_details(a: &Assignments) {
    let mut projects = a.projects.clone();
    projects.sort_by_key(|ref p| p.name.clone());
    for p in &projects {
        let mut students = a.students_for(p.id).clone();
        students.sort_by_key(|&s| a.student(s).name.clone());
        if !students.is_empty() {
            println!("{}:", p.name);
            for s in students {
                print!("  - {}", a.student(s).name);
                if let Some(rank) = a.rank_of(s, p.id) {
                    print!(" (rank {})", rank + 1);
                }
                if a.is_pinned_for(s, p.id) {
                    print!(" (pinned)");
                }
                println!();
            }
            println!();
        }
    }
}

fn display_stats(a: &Assignments) {
    let students = a.students.len();
    let lazy = (0..students)
        .filter(|&s| a.rankings(StudentId(s)).is_empty())
        .count();
    println!(
        "Students registered/unregistered/total: {}/{}/{}",
        students - lazy,
        lazy,
        students
    );
    let ranks = statistics(a);
    let cumul = ranks.iter().scan(0, |s, &r| {
        *s += r;
        Some(*s)
    });
    let total: usize = ranks.iter().sum();
    println!("Final ranking:");
    for (rank, (n, c)) in ranks.iter().zip(cumul).enumerate() {
        if *n != 0 {
            println!(
                "  - rank {}: {} (cumulative {} - {:.2}%)",
                rank + 1,
                n,
                c,
                100.0 * c as f32 / total as f32
            );
        }
    }
}

fn display_empty(a: &Assignments) {
    let projects = a.filter_projects(|p| !a.is_open(p));
    if !projects.is_empty() {
        println!("Empty projects:");
        for p in projects {
            println!("  - {}", a.project(p).name);
        }
    }
}

fn check_pinned_consistency(a: &Assignments) {
    for s in &a.students {
        if let Some(p) = a.rankings(s.id).get(0) {
            if a.is_pinned_for(s.id, *p) && a.project_for(s.id) != Some(*p) {
                warn!(
                    "WARNING: student {} did not get pinned project {}",
                    s.name,
                    a.project(*p).name
                );
            }
        }
    }
}

pub struct Config {
    conf: Ini,
}

impl Config {
    fn load(file_name: &str) -> Result<Config, Error> {
        Ok(Config {
            conf: Ini::load_from_file(file_name).context("cannot load configuration file")?,
        })
    }
}

pub fn get_config(config: &Config, section: &str, key: &str) -> Option<String> {
    config
        .conf
        .section(Some(section.to_owned()))
        .and_then(|s| s.get(key))
        .cloned()
}

fn main() -> Result<(), Error> {
    let matches = App::new("rsolver")
        .about("Automatically assign projects to students")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .args_from_usage(
            "
          -c,--config=[FILE] 'Use FILE file instead of rsolver.ini'
          -n,--dry-run       'Do not write back results to database'
          -v...              'Set verbosity level'",
        )
        .get_matches();
    let level = match matches.occurrences_of("v") {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        _ => "trace",
    };
    flexi_logger::Logger::with_str(format!("rsolver={}", level))
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
    let config = Config::load(matches.value_of("config").unwrap_or("rsolver.ini"))?;
    let dry_run = matches.is_present("dry_run");
    let mut loader =
        match &get_config(&config, "solver", "loader").unwrap_or_else(|| "mysql".to_owned())[..] {
            "mysql" => MysqlLoader::new(&config)?,
            other => bail!("unknown loader: {}", other),
        };
    let mut assignments = loader.load().map(|(s, p)| Assignments::new(s, p))?;
    {
        let mut algo: Box<Algo> = match &get_config(&config, "solver", "algorithm")
            .unwrap_or_else(|| "hungarian".to_owned())[..]
        {
            "ordering" => Box::new(Ordering::new(&mut assignments)),
            "hungarian" => Box::new(Hungarian::new(&mut assignments)),
            other => bail!("unknown algorithm: {}", other),
        };
        algo.assign()?;
    }
    if !dry_run {
        loader.save(&assignments)?;
    }
    display_details(&assignments);
    display_stats(&assignments);
    display_empty(&assignments);
    check_pinned_consistency(&assignments);
    Ok(())
}
