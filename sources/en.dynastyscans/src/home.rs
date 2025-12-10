use aidoku::{
    Home, HomeComponent, HomeLayout, HomePartialResult, alloc::vec,
    imports::std::send_partial_result,
};

use crate::DynastyScans;

impl Home for DynastyScans {
    fn get_home(&self) -> aidoku::Result<HomeLayout> {
        let components = vec![
            HomeComponent {
                title: Some("Popular New Titles".into()),
                subtitle: None,
                value: aidoku::HomeComponentValue::empty_big_scroller(),
            },
            HomeComponent {
                title: Some("Latest Updates".into()),
                subtitle: None,
                value: aidoku::HomeComponentValue::empty_manga_chapter_list(),
            },
        ];

        send_partial_result(&HomePartialResult::Layout(HomeLayout { components }));
        Ok(HomeLayout::default())
    }
}
