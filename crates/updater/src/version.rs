//! Numéros de version façon SemVer (`1.2.3`, `1.2.3-beta.2`, `v1.2.3+build`) : seule leur
//! comparaison sert (une version est une mise à jour si elle est plus grande).
use std::cmp::Ordering;

/// Version comparée selon SemVer 2.0 : majeure, mineure, corrective, puis la pré-version
/// (absente > présente ; identifiants numériques avant les autres) ; les métadonnées de
/// construction (`+…`) sont ignorées.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Version {
    numbers: [u64; 3],
    pre: Vec<String>,
}

impl Version {
    /// `None` si le texte n'est pas une version (`v` initial accepté, parties manquantes
    /// comptées à zéro : `1.2` vaut `1.2.0`).
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let text = text.trim().trim_start_matches('v');
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, pre.split('.').map(str::to_owned).collect()),
            None => (text, Vec::new()),
        };
        let mut numbers = [0; 3];
        let mut parts = core.split('.');
        for slot in &mut numbers {
            if let Some(part) = parts.next() {
                *slot = part.parse().ok()?;
            }
        }
        if parts.next().is_some() || core.is_empty() {
            return None;
        }
        Some(Self { numbers, pre })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.numbers.cmp(&other.numbers).then_with(|| {
            match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => compare_pre(&self.pre, &other.pre),
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn compare_pre(a: &[String], b: &[String]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let order = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.cmp(y),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn versions_compare_like_semver() {
        assert!(v("0.5.0") > v("0.4.4"));
        assert!(v("0.4.10") > v("0.4.9"));
        assert!(v("1.0.0") > v("1.0.0-rc.1"));
        assert!(v("1.0.0-rc.2") > v("1.0.0-rc.1"));
        assert!(v("1.0.0-beta") > v("1.0.0-2"));
        assert!(v("1.0.0-alpha.1") > v("1.0.0-alpha"));
        assert_eq!(v("v1.2.3+build.7"), v("1.2.3"));
        assert_eq!(v("1.2"), v("1.2.0"));
        assert!(Version::parse("abc").is_none());
        assert!(Version::parse("1.2.3.4").is_none());
        assert!(Version::parse("").is_none());
    }
}
