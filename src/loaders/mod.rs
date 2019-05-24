use crate::model::*;
use std::collections::HashMap;

pub use self::loader::Loader;
pub use self::mysql_loader::MysqlLoader;

mod loader;
mod mysql_loader;

fn remap_projects(projects: &mut Vec<Project>) -> HashMap<ProjectId, ProjectId> {
    let map: HashMap<ProjectId, ProjectId> = projects
        .iter()
        .zip(0..)
        .map(|(p, n)| (p.id, ProjectId(n)))
        .collect();
    for project in projects.iter_mut() {
        project.id = map[&project.id];
    }
    map
}

fn remap_students(students: &mut Vec<Student>) {
    for (idx, student) in students.iter_mut().enumerate() {
        student.id = StudentId(idx);
    }
}

fn remap(students: &mut Vec<Student>, projects: &mut Vec<Project>) {
    remap_students(students);
    let map = remap_projects(projects);
    for student in students {
        for id in &mut student.rankings {
            *id = map[&*id];
        }
        student.bonuses = student
            .bonuses
            .iter()
            .map(|(&k, &v)| (map[&k], v))
            .collect();
    }
}
