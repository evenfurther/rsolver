use crate::model::Assignments;
use anyhow::{bail, Error};
use tracing::warn;

pub fn check_pinned_consistency(a: &Assignments) {
    for s in a.all_students() {
        if let Some(p) = a.rankings(s).get(0) {
            if a.is_pinned_for(s, *p) && a.project_for(s) != Some(*p) {
                warn!(
                    student = %a.student(s).name,
                    project = %a.project(*p).name,
                    "student did not get pinned project"
                );
            }
        }
    }
}

pub fn ensure_acceptable(a: &Assignments) -> Result<(), Error> {
    if let Some(unacceptable) = a
        .all_projects()
        .iter()
        .find(|&&p| a.is_open(p) && !a.is_acceptable(p))
    {
        bail!(
            "project {} has an unacceptable number of students",
            a.project(*unacceptable).name
        );
    }
    Ok(())
}
