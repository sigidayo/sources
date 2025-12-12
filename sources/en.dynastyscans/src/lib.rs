#![no_std]

use aidoku::{
    AidokuError, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
    Listing, ListingProvider, Manga, MangaPageResult, Page, PageContext, Source,
    alloc::{
        Vec,
        string::{String, ToString},
    },
    error,
    helpers::uri::QueryParameters,
    imports::net::{Request, TimeUnit::Seconds, set_rate_limit},
    prelude::format,
    println, register_source,
};

use crate::model::{DynastyScansManga, SortingOption};

mod home;
mod model;

const BASE_URL: &str = "https://dynasty-scans.com";
const COVER_QUERY_PARAMETERS_FLAG: &str = "?dsCover";

mod selectors {
    pub const SEARCH_ENTRY: &str = ".chapter-list a.name";
    pub const PAGINATION_COUNT: &str = "div.pagination a";
}

pub struct DynastyScans;

impl Source for DynastyScans {
    fn new() -> Self {
        set_rate_limit(5, 1, Seconds);
        Self
    }

    fn get_search_manga_list(
        &self,
        query: Option<String>,
        page: i32,
        filters: Vec<FilterValue>,
    ) -> aidoku::Result<MangaPageResult> {
        println!("Query: {query:?}\nPage: {page}\nFilters: {filters:#?}");

        let mut sorting_type = SortingOption::default();
        let mut included_tags = Vec::new();
        let mut excluded_tags = Vec::new();

        for filter in filters {
            match filter {
                FilterValue::Sort { id, index, .. } if &id == "Sort" => {
                    sorting_type = index.try_into()?;
                }
                FilterValue::MultiSelect {
                    id,
                    included,
                    excluded,
                } if &id == "Tag" => {
                    included_tags.push(included);
                    excluded_tags.push(excluded);
                }
                _ => {
                    error!("Unsupported filter value: {filter:?}");
                    return Err(AidokuError::Unimplemented);
                }
            }
        }

        let mut query_parameters = QueryParameters::new();
        query_parameters.push("page", Some(&page.to_string()));
        query_parameters.push("q", query.as_deref());
        query_parameters.push("classes[]", Some("Series")); // TODO Change
        included_tags
            .iter()
            .flatten()
            .for_each(|tag| query_parameters.push("with[]", Some(tag)));
        excluded_tags
            .iter()
            .flatten()
            .for_each(|tag| query_parameters.push("without[]", Some(tag)));
        query_parameters.push("sort", sorting_type.into());
        println!("Query parameters: {query_parameters}");

        let url = format!("{BASE_URL}/search?{query_parameters}");
        let html = Request::get(&url)?.html()?;

        let total_pages = html
            .select(selectors::PAGINATION_COUNT)
            .into_iter()
            .flat_map(|el| el.map(|x| x.text()))
            .flatten()
            .filter_map(|t| t.parse::<i32>().ok())
            .max()
            .unwrap_or(1);

        let entries = html
            .select(selectors::SEARCH_ENTRY)
            .into_iter()
            .flatten()
            .filter_map(|element| {
                let title = element.text()?;
                let url = element.attr("href")?;
                Some(Manga {
                    key: url[1..].to_string(),
                    title,
                    cover: Some(format!("{BASE_URL}{}{COVER_QUERY_PARAMETERS_FLAG}", url)),
                    tags: None,
                    ..Default::default()
                })
            })
            .collect();

        Ok(MangaPageResult {
            entries,
            has_next_page: page < total_pages,
        })
    }

    fn get_manga_update(
        &self,
        _manga: Manga,
        _needs_details: bool,
        _needs_chapters: bool,
    ) -> aidoku::Result<Manga> {
        Err(AidokuError::Unimplemented)
    }

    fn get_page_list(&self, _manga: Manga, _chapter: Chapter) -> aidoku::Result<Vec<Page>> {
        Err(AidokuError::Unimplemented)
    }
}

// Because currently in the wasm bindings there is no way to send a partial `MangaPageResult`. The search ui has to wait for all detailed manga requests to get the cover before the ui becomes interactable.
// In this however, we set the cover url as the url to the manga itself with an additional flag and defer loading them until the ui has loaded.
// Then in the implementation below, we can simplify filter for urls with the flag and handle them accordingly.
//
// A rough benchmark of how long it takes to load an interactable ui with images enabled
// - Singular requests: 20~ seconds
// - Concurrent requests: 8~ seconds
// - This method: <1~ second
// Obviously hijacking a trait meant for adding extra headers to image requests is less than ideal, but the pros definitely outweigh the cons.
impl ImageRequestProvider for DynastyScans {
    fn get_image_request(
        &self,
        url: String,
        _context: Option<PageContext>,
    ) -> aidoku::Result<Request> {
        Ok(match url.find(COVER_QUERY_PARAMETERS_FLAG) {
            Some(flag_idx) => {
                let url = format!("{}.json", &url[..flag_idx]);
                let details = Request::get(url)?.json_owned::<DynastyScansManga>()?;
                Request::get(format!(
                    "{BASE_URL}{}",
                    details
                        .cover_url
                        .ok_or(AidokuError::message("Missing cover image"))?
                ))?
            }
            None => Request::get(url)?,
        })
    }
}

impl ListingProvider for DynastyScans {
    fn get_manga_list(&self, _listing: Listing, _page: i32) -> aidoku::Result<MangaPageResult> {
        Err(AidokuError::Unimplemented)
    }
}

impl DeepLinkHandler for DynastyScans {
    fn handle_deep_link(&self, _url: String) -> aidoku::Result<Option<DeepLinkResult>> {
        Err(AidokuError::Unimplemented)
    }
}

register_source!(
    DynastyScans,
    ListingProvider,
    ImageRequestProvider,
    Home,
    DeepLinkHandler
);
