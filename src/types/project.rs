#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ProjectId(pub usize);

#[derive(Clone, Debug)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub min_students: usize,
    pub max_students: usize,
    pub max_occurrences: usize,
}

impl Project {
    pub fn can_host(&self, occ: usize) -> Vec<usize> {
        assert!(occ <= self.max_occurrences);
        if occ == 1 || self.min_students * 2 <= self.max_students {
            (self.min_students..=self.max_students * occ).collect()
        } else {
            let mut r = Vec::new();
            for occurrence in 1..=occ {
                for students in self.min_students * occurrence..=self.max_students * occurrence {
                    r.push(students);
                }
            }
            r
        }
    }

    pub fn acceptable(&self, occ: usize, n: usize) -> bool {
        assert!(occ <= self.max_occurrences);
        (1..=occ).any(|occ| n >= occ * self.min_students && n <= occ * self.max_students)
    }
}

#[test]
fn test_acceptable() {
    let p = Project {
        id: ProjectId(0),
        name: "dummy".into(),
        min_students: 2,
        max_students: 4,
        max_occurrences: 2,
    };
    assert_eq!(
        (1..10).filter(|n| p.acceptable(2, *n)).collect::<Vec<_>>(),
        vec![2, 3, 4, 5, 6, 7, 8]
    );
    let p = Project {
        min_students: 5,
        max_students: 6,
        max_occurrences: 3,
        ..p
    };
    assert_eq!(
        (1..20).filter(|n| p.acceptable(2, *n)).collect::<Vec<_>>(),
        vec![5, 6, 10, 11, 12]
    );
    assert_eq!(
        (1..20).filter(|n| p.acceptable(3, *n)).collect::<Vec<_>>(),
        vec![5, 6, 10, 11, 12, 15, 16, 17, 18]
    );
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
    assert_eq!(p.can_host(2), (2..9).collect::<Vec<_>>());
    let p = Project {
        min_students: 5,
        max_students: 6,
        max_occurrences: 3,
        ..p
    };
    assert_eq!(p.can_host(2), vec![5, 6, 10, 11, 12]);
    assert_eq!(p.can_host(3), vec![5, 6, 10, 11, 12, 15, 16, 17, 18]);
}
