use std::fmt;

use serde::{Deserialize, Serialize};

/// Current version of the protocol, derived from the package version.
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Clone, Copy)]
pub struct Version {
    pub number: &'static str,
    pub code: AisCode,
}

/// Enumeration representing different version codes.
#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Clone, Copy)]
pub enum AisCode {
    /// Production version.
    Production,
    /// Production candidate version.
    ProductionCandidate,
    /// Beta version.
    Beta,
    /// Alpha version.
    Alpha,
    /// Patched
    Patched, // If a quick patch is issued before the platform is updated we can use this code
             // ! This code will ignore compatibility checks BE MINDFUL
}

impl fmt::Display for AisCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ais_code = match self {
            AisCode::Production => "P",
            AisCode::ProductionCandidate => "RC",
            AisCode::Beta => "b",
            AisCode::Alpha => "a",
            AisCode::Patched => "*",
        };
        write!(f, "{}", ais_code)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = self;
        write!(f, "{}{}", data.number, data.code)
    }
}

impl Version {
    /// Get the current version of the ais platform as a filled struct
    pub fn get_raw() -> Self {
        Version {
            number: &VERSION,
            code: AisCode::Patched,
        }
    }

    /// Get the current version String
    pub fn get() -> String {
        let data = Self::get_raw();
        data.to_string()
    }

    /// Checks if a version number given is compatible with the current version
    pub fn comp_raw(incoming: Self) -> bool {
        match (incoming.code, Self::get_raw().code) {
            // (AisCode::Alpha, _) | (_, AisCode::Alpha) => true,
            (AisCode::Alpha, AisCode::Alpha) => true,
            (AisCode::Beta, AisCode::Beta)
            | (AisCode::Beta, AisCode::Alpha)
            | (AisCode::Alpha, AisCode::Beta) => true,
            (AisCode::ProductionCandidate, AisCode::ProductionCandidate)
            | (AisCode::ProductionCandidate, AisCode::Beta)
            | (AisCode::Beta, AisCode::ProductionCandidate) => {
                let (inc_major, _) = Self::parse_version(&incoming.number).unwrap();
                let (ver_major, _) = Self::parse_version(VERSION).unwrap();
                inc_major == ver_major
            }
            (AisCode::Production, AisCode::ProductionCandidate)
            | (AisCode::ProductionCandidate, AisCode::Production)
            | (AisCode::Production, AisCode::Production) => {
                let (inc_major, inc_minor) = Self::parse_version(&incoming.number).unwrap();
                let (ver_major, ver_minor) = Self::parse_version(VERSION).unwrap();
                inc_major == ver_major && inc_minor == ver_minor
            }
            _ => false,
        }
    }

    pub fn comp(data: String) -> bool {
        let version = match Self::from_string(data) {
            Some(d) => d,
            None => return false,
        };
        Self::comp_raw(version)
    }

    pub fn to_string(self) -> String {
        let data = self;
        format!("{}{}", data.number, data.code)
    }

    /// Converts a received string into a Version struct
    pub fn from_string(s: String) -> Option<Self> {
        // Find the position of the first non-digit character after the version number
        let pos = s.chars().position(|c| !c.is_digit(10) && c != '.');
        if let Some(pos) = pos {
            let number = &s[..pos];
            let code_str = &s[pos..];
            let code = match code_str {
                "P" => AisCode::Production,
                "RC" => AisCode::ProductionCandidate,
                "b" => AisCode::Beta,
                "a" => AisCode::Alpha,
                "*" => AisCode::Patched,
                _ => return None,
            };
            // Convert the string to a 'static str
            let number_static = Box::leak(number.to_string().into_boxed_str());
            Some(Version {
                number: number_static,
                code,
            })
        } else {
            None
        }
    }

    fn parse_version(v: &str) -> Option<(u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major: u32 = parts[0].parse::<u32>().ok()?;
        let minor: u32 = parts[1].parse::<u32>().ok()?;
        Some((major, minor))
    }
}
