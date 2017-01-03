use project::Project;
use student::Student;
use std::collections::HashMap;

pub use self::loader::Loader;
pub use self::mysql_loader::MysqlLoader;

mod mysql_loader;
mod loader;

fn remap_projects(projects: &mut Vec<Project>) -> HashMap<usize, usize> {
    let mut map: HashMap<usize, usize> = projects.iter().map(|p| p.id).zip(0..).collect();
    for project in projects.iter_mut() {
        project.id = map[&project.id];
    }
    map
}

pub fn remap(students: &mut Vec<Student>, projects: &mut Vec<Project>) {
    let map = remap_projects(projects);
    for student in students.iter_mut() {
        for id in student.rankings.iter_mut() {
            *id = map[&*id];
        }
        for &mut (ref mut id, _) in student.bonuses.iter_mut() {
            *id = map[&*id];
        }
    }
}
