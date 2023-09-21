#![warn(clippy::pedantic)]
#![allow(clippy::cast_possible_truncation)]

use crate::model::Assignments;
use clap::{
    ArgAction::{Count, SetFalse, SetTrue},
    Parser,
};
use eyre::{ensure, Context};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::Level;

mod checks;
mod display;
mod hungarian;
mod loaders;
mod model;
mod remap;
mod stats;

#[derive(Parser)]
#[clap(version, author, about)]
#[allow(clippy::struct_excessive_bools)]
struct Options {
    /// Use FILE instead of rsolver.ini
    #[clap(short, long, value_parser)]
    config: Option<PathBuf>,
    /// Output assignments as CSV records
    ///
    /// The CSV records will be output on the standard output instead
    /// of the plain text assignment.
    #[clap(short = 'C', long, action = SetTrue)]
    csv: bool,
    /// Do not assign unregistered students to any project
    ///
    /// Unregistered students will be dropped from the system.
    /// Be careful in that not enough registered students may fail
    /// to be assigned to projects due to insufficient project members.
    #[clap(short, long, action = SetTrue)]
    drop_unregistered: bool,
    /// Do not write back results to database
    #[clap(short = 'n', long = "dry_run", action = SetFalse)]
    commit_to_db: bool,
    /// Rename lazy student into Zzz + order
    #[clap(short, long, action = SetTrue)]
    rename_unregistered: bool,
    /// Set verbosity level
    ///
    /// This option can be repeated.
    #[clap(short, action = Count)]
    verbosity: u8,
}

#[derive(Deserialize)]
pub struct Config {
    pub solver: SolverConfig,
    pub hungarian: hungarian::Config,
}

#[derive(Deserialize)]
pub struct SolverConfig {
    pub database: String,
}

impl Config {
    fn load<P: AsRef<Path>>(file_name: P) -> eyre::Result<Config> {
        toml::from_str(
            &std::fs::read_to_string(file_name).context("cannot read configuration file")?,
        )
        .context("invalid configuration file")
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let options = Options::parse();
    let level = match options.verbosity {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        _ => Level::TRACE,
    };
    tracing_subscriber::fmt::fmt().with_max_level(level).init();
    let config = Config::load(options.config.unwrap_or(PathBuf::from_str("rsolver.ini")?))?;
    let mut loader = loaders::Loader::new(&config.solver.database).await?;
    // Load data from the database
    let (original_students, original_projects) = loader.load().await?;
    // Isolate lazy students before remapping if asked to do so
    let (original_students, lazy_students) = if options.drop_unregistered {
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
    let mut assignments = Assignments::new(students, projects);
    hungarian::assign(&mut assignments, &config.hungarian)?;
    // Save the results if requested
    if options.commit_to_db {
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
            .filter_students(|s| unassigned_students.binary_search(&s).is_err())
            .into_iter()
            .map(|s| {
                (
                    original_students[s.0].id,
                    original_projects[assignments.project_for(s).unwrap().0].id,
                )
            })
            .collect::<Vec<_>>();
        // Save the assignments and non-assignments into the database
        loader
            .save_assignments(&assignments, &unassigned_students)
            .await?;
    }
    // If CSV output is requested, only output assignments
    if options.csv {
        display::display_csv(&assignments)?;
    } else {
        // Rename lazy students if requested, to ease output comparison
        display::display_details(&assignments, options.rename_unregistered);
        display::display_stats(&assignments, lazy_students.len());
        display::display_missed_bonuses(&assignments);
        display::display_empty(&assignments);
        display::display_with_many_lazy(&assignments);
    }
    checks::check_pinned_consistency(&assignments);
    ensure!(
        assignments.unassigned_students().is_empty(),
        "{n} students could not get assigned to any project",
        n = assignments.unassigned_students().len()
    );
    checks::ensure_acceptable(&assignments)
}
