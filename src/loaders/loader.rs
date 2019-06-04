use crate::model::*;
use failure::{bail, Error, ResultExt};

pub trait Loader {
    fn load_projects(&self) -> Result<Vec<Project>, Error> {
        bail!("implementation needed")
    }

    fn load_students(&self) -> Result<Vec<Student>, Error> {
        bail!("implementation needed")
    }

    fn load_bonuses(&self) -> Result<Vec<(StudentId, ProjectId, isize)>, Error> {
        bail!("implementation needed")
    }

    fn load_preferences(&self) -> Result<Vec<(StudentId, ProjectId, isize)>, Error> {
        bail!("implementation needed")
    }

    fn store_projects(&mut self, _projects: &[Project]) {
        // Do nothing
    }

    fn store_students(&mut self, _students: &[Student]) {
        // Do nothing
    }

    fn load(&mut self) -> Result<(Vec<Student>, Vec<Project>), Error> {
        let mut projects = self.load_projects().context("cannot load projects")?;
        self.store_projects(&projects.clone());
        let mut students = self.load_students().context("cannot load students")?;
        self.store_students(&students.clone());
        let preferences = self.load_preferences().context("cannot load rankings")?;
        let bonuses = self.load_bonuses().context("cannot load bonuses")?;
        for student in &mut students {
            let mut preferences = preferences
                .iter()
                .filter_map(|&(s, p, w)| if s == student.id { Some((p, w)) } else { None })
                .collect::<Vec<_>>();
            preferences.sort_by_key(|&(_, w)| w);
            student.rankings = preferences.into_iter().map(|(p, _)| p).collect();
            student.bonuses = bonuses
                .iter()
                .filter_map(|&(s, p, w)| if s == student.id { Some((p, -w)) } else { None })
                .collect();
        }
        super::remap(&mut students, &mut projects);
        Ok((students, projects))
    }

    fn save(&self, assignments: &Assignments) -> Result<(), Error>;
}
