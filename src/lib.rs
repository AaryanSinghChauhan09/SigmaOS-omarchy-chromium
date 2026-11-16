// SPDX-License-Identifier: Apache-2.0
//! SigmaOS-omarchy-chromium: omarchy chromium component model.
//!
//! Real implementation: chromiums, events, validation, deterministic storage.
//! Replaces the placeholder stub.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chromium {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    pub subject: String,
    pub detail: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmarchyChromiumError {
    EmptyChromiumName,
    DuplicateChromium(String),
    EmptyEventSubject,
}

impl fmt::Display for OmarchyChromiumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyChromiumName => write!(f, "chromium name must not be empty"),
            Self::DuplicateChromium(id) => write!(f, "duplicate chromium {:?}", id),
            Self::EmptyEventSubject => write!(f, "event subject must not be empty"),
        }
    }
}

impl std::error::Error for OmarchyChromiumError {}

pub struct OmarchyChromium {
    pub name: String,
    chromiums: BTreeMap<String, Chromium>,
    events: BTreeMap<String, Event>,
}

impl OmarchyChromium {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            chromiums: BTreeMap::new(),
            events: BTreeMap::new(),
        }
    }

    pub fn add_chromium(&mut self, item: Chromium) -> Result<(), OmarchyChromiumError> {
        if item.name.trim().is_empty() {
            return Err(OmarchyChromiumError::EmptyChromiumName);
        }
        let id = item.id.clone();
        if self.chromiums.contains_key(&id) {
            return Err(OmarchyChromiumError::DuplicateChromium(id.clone()));
        }
        self.chromiums.insert(id, item);
        Ok(())
    }

    pub fn chromium_count(&self) -> usize {
        self.chromiums.len()
    }

    pub fn get_chromium(&self, id: &str) -> Option<&Chromium> {
        self.chromiums.get(id)
    }

    pub fn add_event(&mut self, event: Event) -> Result<(), OmarchyChromiumError> {
        if event.subject.trim().is_empty() {
            return Err(OmarchyChromiumError::EmptyEventSubject);
        }
        self.events.insert(event.id.clone(), event);
        Ok(())
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn get_event(&self, id: &str) -> Option<&Event> {
        self.events.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_chromium(id: &str, name: &str) -> Chromium {
        Chromium {
            id: id.into(),
            name: name.into(),
            kind: "default".into(),
            detail: String::new(),
        }
    }

    #[test]
    fn test_create() {
        let m = OmarchyChromium::new("OmarchyChromium");
        assert_eq!(m.name, "OmarchyChromium");
        assert_eq!(m.chromium_count(), 0);
        assert_eq!(m.event_count(), 0);
    }

    #[test]
    fn test_add_chromium() {
        let mut m = OmarchyChromium::new("OmarchyChromium");
        m.add_chromium(sample_chromium("a1", "Alpha")).unwrap();
        assert_eq!(m.chromium_count(), 1);
        assert!(m.get_chromium("a1").is_some());
    }

    #[test]
    fn test_empty_name_rejected() {
        let mut m = OmarchyChromium::new("OmarchyChromium");
        assert!(m.add_chromium(sample_chromium("x", "   ")).is_err());
    }

    #[test]
    fn test_duplicate_rejected() {
        let mut m = OmarchyChromium::new("OmarchyChromium");
        m.add_chromium(sample_chromium("x", "One")).unwrap();
        assert!(m.add_chromium(sample_chromium("x", "Two")).is_err());
    }

    #[test]
    fn test_event_validation() {
        let mut m = OmarchyChromium::new("OmarchyChromium");
        let bad = Event { id: "e1".into(), subject: "".into(), detail: String::new(), timestamp: 0 };
        assert!(m.add_event(bad).is_err());
        let good = Event { id: "e1".into(), subject: "boot".into(), detail: String::new(), timestamp: 1 };
        m.add_event(good).unwrap();
        assert_eq!(m.event_count(), 1);
    }
}
