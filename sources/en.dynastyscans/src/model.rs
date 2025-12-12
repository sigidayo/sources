use core::fmt::{Display, Formatter};

use aidoku::{AidokuError, Manga, alloc::String, prelude::format};
use serde::Deserialize;

use crate::BASE_URL;

#[derive(Debug, Deserialize)]
pub struct DynastyScansManga {
    pub name: String,
    pub permalink: String,
    pub r#type: DynastyScansMangaType,
    #[serde(rename = "cover")]
    pub cover_url: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub enum DynastyScansMangaType {
    Anthology,
    Doujin,
    Series,
}

#[derive(Debug, Default)]
pub enum SortingOption {
    Alphabetical,
    BestMatch,
    DateAdded,
    #[default]
    ReleaseDate,
}

impl From<DynastyScansManga> for Manga {
    fn from(val: DynastyScansManga) -> Manga {
        Manga {
            url: Some(format!("{BASE_URL}/{}/{}", val.r#type, val.permalink)),
            key: val.permalink,
            title: val.name,
            cover: val.cover_url.map(|url| format!("{BASE_URL}{}", url)),
            artists: None,
            authors: None,
            description: val.description,
            tags: None,
            status: Default::default(),
            content_rating: Default::default(),
            viewer: Default::default(),
            update_strategy: Default::default(),
            next_update_time: None,
            chapters: None,
        }
    }
}

impl Display for DynastyScansMangaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            DynastyScansMangaType::Anthology => write!(f, "anthology"),
            DynastyScansMangaType::Doujin => write!(f, "doujin"),
            DynastyScansMangaType::Series => write!(f, "series"),
        }
    }
}

impl TryFrom<i32> for SortingOption {
    type Error = AidokuError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SortingOption::Alphabetical),
            1 => Ok(SortingOption::BestMatch),
            2 => Ok(SortingOption::DateAdded),
            3 => Ok(SortingOption::ReleaseDate),
            _ => Err(AidokuError::Unimplemented),
        }
    }
}

impl From<SortingOption> for Option<&str> {
    fn from(val: SortingOption) -> Option<&'static str> {
        match val {
            SortingOption::Alphabetical => Some("name"),
            SortingOption::BestMatch => None,
            SortingOption::DateAdded => Some("created_at"),
            SortingOption::ReleaseDate => Some("released_on"),
        }
    }
}
