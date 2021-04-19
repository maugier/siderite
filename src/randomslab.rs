use slab;
use fastrand;

type Label = [u8; 8];

pub struct Slab<T>(slab::Slab<(Label, T)>);

fn random_label() -> Label {
    let mut r = [0; 8];
    for b in r.iter_mut() {
        *b = fastrand::alphabetic() as u8;
    }
    r
}

fn split2(s: &str) -> Option<(usize, &str)> {
    let mut split = s.splitn(2, ':');
    let one = split.next()?;
    let two = split.next()?;
    if split.next().is_some() { 
        return None;
    }
    let one = one.parse().ok()?;
    Some((one, two))
}

impl<T> Slab<T> {

    pub fn new() -> Self {
        Self(slab::Slab::new())
    }

    pub fn insert(&mut self, t: T) -> String {
        let label = random_label();
        let idx = self.0.insert((label, t));
        format!("{}:{}", idx, std::str::from_utf8(&label).unwrap())
    }

    pub fn get(&self, key: &str) -> Option<&T> {
        let (n, label) = split2(key)?;
        let entry = self.0.get(n)?;
        if entry.0 == label.as_bytes() {
            Some(&entry.1)
        } else {
            None
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<T> {
        let (n, label) = split2(key)?;

        if &self.0.get(n)?.0 == label.as_bytes() {
            Some(self.0.remove(n).1)
        } else {
            None
        }

    }

}

#[test]
fn test_random_slab() {

    let mut slab = Slab::new();

    let l1 = slab.insert("abc");
    eprintln!("label of abc is {}", l1);
    let l2 = slab.insert("def");

    assert_eq!(slab.remove("0:nonsense"), None);

    assert_eq!(slab.remove(&l1), Some("abc"));   
    assert_eq!(slab.remove(&l1), None);
    assert_eq!(slab.remove(&l1), None);

    let l3 = slab.insert("ghi");
    assert_eq!(slab.remove(&l3), Some("ghi"));
    assert_eq!(slab.remove(&l3), None);

    assert_eq!(slab.remove(&l2), Some("def"));
    assert_eq!(slab.remove("nonsense"), None);

}