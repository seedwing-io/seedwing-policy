use crate::package::Package;
use crate::runtime::PackagePath;

mod find_advisory;
mod from_cve;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["rhsa"]));
    pkg.register_function("from-cve".into(), from_cve::FromCve);
    pkg.register_function("find-advisory".into(), find_advisory::FindAdvisory);
    pkg
}

#[allow(clippy::upper_case_acronyms)]
pub enum AdvisoryId {
    RHSA { year: u32, number: u32 },
    RHBA { year: u32, number: u32 },
}

impl AdvisoryId {
    pub fn unwrap(&self) -> (&str, u32, u32) {
        match self {
            Self::RHSA { year, number } => ("rhsa", *year, *number),
            Self::RHBA { year, number } => ("rhba", *year, *number),
        }
    }
}

impl core::str::FromStr for AdvisoryId {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let is_rhsa = s.starts_with("RHSA-");
        let is_rhba = s.starts_with("RHBA-");
        if is_rhsa || is_rhba {
            let mut parts = s[5..].split(':');
            if let Some(year) = parts.next() {
                if let Some(number) = parts.next() {
                    let year = year.parse::<u32>().map_err(|_| ())?;
                    let number = number.parse::<u32>().map_err(|_| ())?;
                    if is_rhsa {
                        return Ok(Self::RHSA { year, number });
                    } else {
                        return Ok(Self::RHBA { year, number });
                    }
                }
            }
        }
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_id() {
        assert!("RHSA-2022:1234".parse::<AdvisoryId>().is_ok());
        assert!("RHBA-2022:1234".parse::<AdvisoryId>().is_ok());
        assert!(!"rhsa-2022:1234".parse::<AdvisoryId>().is_ok());
        assert!(!"RHSA-2022::1234".parse::<AdvisoryId>().is_ok());
        assert!(!"RHSA2022:1234".parse::<AdvisoryId>().is_ok());
    }
}
