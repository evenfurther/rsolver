use crate::model::{Project, ProjectId, Student, StudentId};
use anyhow::{bail, Context, Error};
use async_trait::async_trait;
use tracing::trace;

#[async_trait]
pub trait Loader: Send + Sync {
    async fn load_projects(&self) -> Result<Vec<Project>, Error> {
        bail!("implementation needed")
    }

    async fn load_students(&self) -> Result<Vec<Student>, Error> {
        bail!("implementation needed")
    }

    async fn load_bonuses(&self) -> Result<Vec<(StudentId, ProjectId, i64)>, Error> {
        bail!("implementation needed")
    }

    async fn load_preferences(&self) -> Result<Vec<(StudentId, ProjectId, i64)>, Error> {
        bail!("implementation needed")
    }

    async fn load(&mut self) -> Result<(Vec<Student>, Vec<Project>), Error> {
        let projects = self.load_projects().await.context("cannot load projects")?;
        let mut students = self.load_students().await.context("cannot load students")?;
        let preferences = self
            .load_preferences()
            .await
            .context("cannot load rankings")?;
        let bonuses = self.load_bonuses().await.context("cannot load bonuses")?;
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
            if !student.bonuses.is_empty() {
                trace!("{} has been assigned the following bonuses:", student.name);
                for (p, w) in &student.bonuses {
                    trace!("  - {}: {}", projects[p.0].name, w);
                }
            }
        }
        Ok((students, projects))
    }

    async fn save_assignments(
        &self,
        assignments: &[(StudentId, ProjectId)],
        unassigned: &[StudentId],
    ) -> Result<(), Error>;
}
