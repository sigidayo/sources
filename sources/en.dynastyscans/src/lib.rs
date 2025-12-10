#![no_std]

use aidoku::{
    AidokuError, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Listing, ListingProvider,
    Manga, MangaPageResult, Page, Source,
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

use crate::{
    model::{DynastyScansManga, SortingOption},
    net::BatchedRequest,
};

mod home;
mod model;
mod net;

const BASE_URL: &str = "https://dynasty-scans.com";

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
            .select("div.pagination a")
            .into_iter()
            .flat_map(|el| el.map(|x| x.text()))
            .flatten()
            .filter_map(|t| t.parse::<i32>().ok())
            .max()
            .unwrap_or(1);

        let request_urls: Vec<String> = html
            .select(".chapter-list a.name")
            .into_iter()
            .flatten()
            .filter_map(|el| el.attr("href"))
            .map(|href| format!("{BASE_URL}{href}.json"))
            .collect();

        let responses: Vec<DynastyScansManga> = BatchedRequest::new(request_urls).get_jsons()?;

        Ok(MangaPageResult {
            entries: responses.into_iter().map(|m| m.into()).collect(),
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

register_source!(DynastyScans, ListingProvider, Home, DeepLinkHandler);
