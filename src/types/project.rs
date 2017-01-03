#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ProjectId(pub usize);

#[derive(Debug)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub min_students: usize,
    pub max_students: usize,
    pub max_occurrences: usize,
}

impl Project {
    pub fn can_host(&self) -> Vec<usize> {
        if self.max_occurrences == 1 || self.min_students * 2 <= self.max_students {
            (self.min_students..self.max_students * self.max_occurrences + 1).collect()
        } else {
            let mut r = Vec::new();
            for occurrence in 1..self.max_occurrences + 1 {
                for students in self.min_students * occurrence..self.max_students * occurrence + 1 {
                    r.push(students);
                }
            }
            r
        }
    }
}

#[test]
fn test_can_host() {
    let p = Project {
        id: ProjectId(0),
        name: "dummy".into(),
        min_students: 2,
        max_students: 4,
        max_occurrences: 2,
    };
    assert_eq!(p.can_host(), (2..9).collect::<Vec<_>>());
    let p = Project {
        min_students: 5,
        max_students: 6,
        ..p
    };
    assert_eq!(p.can_host(), vec![5, 6, 10, 11, 12]);
    let p = Project { max_occurrences: 3, ..p };
    assert_eq!(p.can_host(), vec![5, 6, 10, 11, 12, 15, 16, 17, 18]);
}
