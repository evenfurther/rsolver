use anyhow::{ensure, Error};
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::Level;

use config::{get_config, Config};
use hungarian::Hungarian;
use model::{Assignments, Project, Student};

mod checks;
mod config;
mod display;
mod hungarian;
mod loaders;
mod model;
mod remap;
mod stats;

#[tracing::instrument(skip_all)]
fn assign(
    students: Vec<Student>,
    projects: Vec<Project>,
    config: &Config,
) -> Result<Assignments, Error> {
    let start = std::time::Instant::now();
    let mut assignments = Assignments::new(students, projects);
    Hungarian::new(&mut assignments, config)?.assign()?;
    tracing::debug!(elapsed = ?start.elapsed(), "assignments computation time");
    Ok(assignments)
}

#[derive(Parser)]
#[clap(version, author, about)]
struct Options {
    /// Use FILE instead of rsolver.ini
    #[clap(short, long, parse(from_os_str))]
    config: Option<PathBuf>,
    /// Output assignments as CSV records
    ///
    /// The CSV records will be output on the standard output instead
    /// of the plain text assignment.
    #[clap(short = 'C', long)]
    csv: bool,
    /// Do not assign unregistered students to any project
    ///
    /// Unregistered students will be dropped from the system.
    /// Be careful in that not enough registered students may fail
    /// to be assigned to projects due to insufficient project members.
    #[clap(short, long)]
    drop_unregistered: bool,
    /// Do not write back results to database
    #[clap(short = 'n', long)]
    dry_run: bool,
    /// Rename lazy student into Zzz + order
    #[clap(short, long)]
    rename_unregistered: bool,
    /// Set verbosity level
    ///
    /// This option can be repeated.
    #[clap(short, parse(from_occurrences))]
    verbosity: usize,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
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
    let mut loader =
        loaders::Loader::new(&get_config(&config, "solver", "database").unwrap()).await?;
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
    let assignments = assign(students, projects, &config)?;
    if !options.dry_run {
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
