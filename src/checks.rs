use crate::model::Assignments;
use eyre::bail;
use tracing::warn;

pub fn check_pinned_consistency(a: &Assignments) {
    for s in a.all_students() {
        if let Some(p) = a.rankings(s).first() {
            if a.is_pinned_for(s, *p) && a.project_for(s) != Some(*p) {
                warn!(
                    student = %a.student(s),
                    project = %a.project(*p),
                    "student did not get pinned project"
                );
            }
        }
    }
}

pub fn ensure_acceptable(a: &Assignments) -> eyre::Result<()> {
    if let Some(unacceptable) = a
        .all_projects()
        .iter()
        .find(|&&p| a.is_open(p) && !a.is_acceptable(p))
    {
        bail!(
            "project {name} has an unacceptable number of students",
            name = a.project(*unacceptable).name
        );
    }
    Ok(())
}
