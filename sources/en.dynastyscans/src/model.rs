use core::fmt::{Display, Formatter};

use aidoku::{
    AidokuError, Chapter, ContentRating, Manga, MangaStatus, Page, PageContent, UpdateStrategy,
    alloc::{String, Vec, string::ToString, vec},
    imports::{html::Html, std},
    prelude::format,
};
use serde::{Deserialize, Deserializer, de::IgnoredAny};

use crate::BASE_URL;

#[derive(Debug, Deserialize)]
pub struct DynastyScansManga {
    pub name: String,
    pub permalink: String,
    pub r#type: DynastyScansMangaType,
    #[serde(rename = "cover")]
    pub cover_url: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "taggings", deserialize_with = "deserialize_chapters")]
    pub chapters: Vec<Chapter>,
    pub tags: Vec<DynastyScansTag>,
}

#[derive(Debug, Deserialize)]
pub enum DynastyScansMangaType {
    Anthology,
    Doujin,
    Series,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DynastyScansTag {
    Author { name: String },
    General { name: String },
    Status { name: String },
}

#[derive(Debug, Deserialize)]
pub struct DynastyScansChapter {
    pub pages: Vec<DynastyScansPage>,
}

#[derive(Debug, Deserialize)]
pub struct DynastyScansPage {
    pub url: String,
}

#[derive(Debug, Default)]
pub enum SortingOption {
    Alphabetical,
    BestMatch,
    DateAdded,
    #[default]
    ReleaseDate,
}

impl DynastyScansManga {
    fn content_rating(&self) -> ContentRating {
        let mut rating = ContentRating::Safe;
        let general_tags = &self
            .tags
            .iter()
            .filter(|t| matches!(t, DynastyScansTag::General { .. }))
            .collect::<Vec<_>>();
        for tag in general_tags {
            match tag.name() {
                "NSFW" => rating = ContentRating::NSFW,
                "Ecchi" if rating != ContentRating::NSFW => rating = ContentRating::Suggestive,
                _ => continue,
            }
        }
        rating
    }
    fn status(&self) -> MangaStatus {
        self.tags
            .iter()
            .find(|t| matches!(t, DynastyScansTag::Status { .. }))
            .map(|t| match t.name() {
                "Completed" => MangaStatus::Completed,
                "Ongoing" => MangaStatus::Ongoing,
                "Cancelled" => MangaStatus::Cancelled,
                "On Hiatus" => MangaStatus::Hiatus,
                _ => MangaStatus::Unknown,
            })
            .unwrap_or(MangaStatus::Unknown)
    }
}

impl DynastyScansTag {
    fn name(&self) -> &str {
        match self {
            DynastyScansTag::Author { name } => name,
            DynastyScansTag::General { name } => name,
            DynastyScansTag::Status { name } => name,
        }
    }
}

impl From<DynastyScansManga> for Manga {
    fn from(val: DynastyScansManga) -> Manga {
        let status = val.status();

        Manga {
            cover: val
                .cover_url
                .as_ref()
                .map(|url| format!("{BASE_URL}{}", *url)),
            authors: val
                .tags
                .iter()
                .find(|t| matches!(t, DynastyScansTag::Author { .. }))
                .map(|t| vec![t.name().to_string()]),
            description: val.description.as_ref().and_then(Html::unescape),
            url: Some(format!("{BASE_URL}/{}/{}", val.r#type, val.permalink)),
            tags: Some(
                val.tags
                    .iter()
                    .filter_map(|t| match t {
                        DynastyScansTag::General { name } => Some(name),
                        _ => None,
                    })
                    .cloned()
                    .collect(),
            ),
            status,
            content_rating: val.content_rating(),
            viewer: Default::default(), // TODO deduce preferred viewer from tags
            update_strategy: match status {
                MangaStatus::Completed | MangaStatus::Cancelled => UpdateStrategy::Never,
                _ => UpdateStrategy::Always,
            },
            chapters: Some(val.chapters),
            key: format!("{}/{}", val.r#type, val.permalink),
            title: val.name,
            ..Default::default()
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

impl From<DynastyScansPage> for Page {
    fn from(value: DynastyScansPage) -> Self {
        Page {
            content: PageContent::Url(format!("{BASE_URL}{}", value.url), None),
            ..Default::default()
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

fn deserialize_chapters<'de, D>(data: D) -> Result<Vec<Chapter>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Inner {
        Header {
            #[allow(dead_code)]
            header: IgnoredAny,
        },
        Entry {
            title: String,
            permalink: String,
            #[serde(deserialize_with = "deserialize_date")]
            released_on: i64,
        },
    }

    fn deserialize_date<'de, D>(data: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        std::parse_date(String::deserialize(data)?, "yyyy-MM-dd")
            .ok_or(serde::de::Error::custom("Invalid date"))
    }

    let json = Vec::<Inner>::deserialize(data)?;

    let mut ret = Vec::new();
    let mut volume = 0;
    let mut chapter = 0;
    for inner in json {
        match inner {
            Inner::Header { .. } => volume += 1,
            Inner::Entry {
                title,
                permalink,
                released_on,
            } => {
                chapter += 1;
                ret.push(Chapter {
                    url: Some(format!("{BASE_URL}/chapters/{permalink}")),
                    key: permalink,
                    title: Some(title),
                    chapter_number: Some(chapter as f32),
                    volume_number: Some(volume as f32),
                    date_uploaded: Some(released_on),
                    ..Default::default()
                })
            }
        }
    }
    ret.reverse();

    Ok(ret)
}
