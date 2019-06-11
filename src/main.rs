#[macro_use]
extern crate log;

use crate::algos::*;
use crate::config::{get_config, Config};
use crate::loaders::*;
use crate::model::*;
use clap::{crate_authors, crate_version, App};
use failure::{bail, ensure, Error};
use flexi_logger;

mod algos;
mod checks;
mod config;
mod display;
mod loaders;
mod model;
mod remap;
mod stats;

fn assign(
    students: Vec<Student>,
    projects: Vec<Project>,
    config: &Config,
) -> Result<Assignments, Error> {
    let mut assignments = Assignments::new(students, projects);
    {
        let mut algo: Box<dyn Algo> = match &get_config(config, "solver", "algorithm")
            .unwrap_or_else(|| "hungarian".to_owned())[..]
        {
            "ordering" => Box::new(Ordering::new(&mut assignments)),
            "hungarian" => Box::new(Hungarian::new(&mut assignments, config)?),
            other => bail!("unknown algorithm: {}", other),
        };
        algo.assign()?;
    }
    Ok(assignments)
}

fn main() -> Result<(), Error> {
    let matches = App::new("rsolver")
        .about("Automatically assign projects to students")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .args_from_usage(
            "
          -c,--config=[FILE]        'Use FILE file instead of rsolver.ini'
          -d,--drop-unregistered    'Do not assign unregistered students to any project'
          -n,--dry-run              'Do not write back results to database'
          -r,--rename-unregistered  'Rename lazy student into Zzz + order'
          -v...                     'Set verbosity level'",
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
    let mut loader: Box<dyn Loader> =
        match &get_config(&config, "solver", "loader").unwrap_or_else(|| "mysql".to_owned())[..] {
            "mysql" => Box::new(MysqlLoader::new(&config)?),
            #[cfg(feature = "sqlite")]
            "sqlite" => Box::new(SqliteLoader::new(&config)?),
            other => bail!("unknown loader: {}", other),
        };
    // Load data from the database
    let (original_students, original_projects) = loader.load()?;
    // Isolate lazy students before remapping if asked to do so
    let (original_students, lazy_students) = if matches.is_present("drop-unregistered") {
        remap::separate_lazy(original_students)
    } else {
        (original_students, vec![])
    };
    // Remap students and projects into contiguous values for the algorithm sake
    let (students, projects) = {
        let (mut students, mut projects) = (original_students.clone(), original_projects.clone());
        // Work with normalized values (students and projets starting at 0 and without gaps)
        remap::remap(&mut students, &mut projects);
        (students, projects)
    };
    // Compute the new assignments
    let assignments = assign(students, projects, &config)?;
    if !dry_run {
        // Make a list of unassigned students, be it from the algorithm
        // or because lazy students were singled out beforehand
        let mut unassigned_students = assignments
            .unassigned_students()
            .iter()
            .map(|s| original_students[s.0].id)
            .collect::<Vec<_>>();
        unassigned_students.append(&mut lazy_students.clone());
        unassigned_students.sort();
        // Other students, i.e. assigned students
        let assignments = assignments
            .students
            .iter()
            .filter(|s| unassigned_students.binary_search(&s.id).is_err())
            .map(|s| {
                (
                    original_students[s.id.0].id,
                    original_projects[assignments.project_for(s.id).unwrap().0].id,
                )
            })
            .collect::<Vec<_>>();
        // Save the assignments and non-assignments into the database
        loader.save_assignments(&assignments, &unassigned_students)?
    }
    // Rename lazy students if requested, to ease output comparison
    display::display_details(&assignments, matches.is_present("rename-unregistered"));
    display::display_stats(&assignments, lazy_students.len());
    display::display_empty(&assignments);
    display::display_with_many_lazy(&assignments);
    checks::check_pinned_consistency(&assignments);
    ensure!(
        assignments.unassigned_students().is_empty(),
        "{} students could not get assigned to any project",
        assignments.unassigned_students().len()
    );
    checks::ensure_acceptable(&assignments)
}
