//! Platform view handler

use crossterm::event::{KeyCode, KeyEvent};
use tui_scrollview::ScrollViewState;

use crate::app::types::PlatformView;
use crate::app::App;

impl App {
    /// Handle Platform view keys
    pub(crate) async fn handle_platform_key(&mut self, key: KeyEvent) {
        // For PrivateComponents/Registry/Templates views: j/k scroll description, arrow keys navigate list
        // For Services view: both j/k and arrow keys navigate list
        match key.code {
            // j/k keys - scroll description in package/template views, navigate list in Services
            KeyCode::Char('j') => match self.platform_view {
                PlatformView::Services => self.services_list.next(),
                PlatformView::PrivateComponents
                | PlatformView::Registry
                | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_down();
                }
            },
            KeyCode::Char('k') => match self.platform_view {
                PlatformView::Services => self.services_list.previous(),
                PlatformView::PrivateComponents
                | PlatformView::Registry
                | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_up();
                }
            },
            // J/K for page scroll in description
            KeyCode::Char('J') => match self.platform_view {
                PlatformView::Services => {}
                PlatformView::PrivateComponents
                | PlatformView::Registry
                | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_page_down();
                }
            },
            KeyCode::Char('K') => match self.platform_view {
                PlatformView::Services => {}
                PlatformView::PrivateComponents
                | PlatformView::Registry
                | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_page_up();
                }
            },
            // Arrow keys - always navigate the list
            KeyCode::Up => match self.platform_view {
                PlatformView::Services => self.services_list.previous(),
                PlatformView::PrivateComponents => {
                    self.private_packages_list.previous();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.schedule_readme_load();
                }
                PlatformView::Registry => {
                    self.packages_list.previous();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.schedule_readme_load();
                }
                PlatformView::Templates => {
                    self.templates_list.previous();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                }
            },
            KeyCode::Down => match self.platform_view {
                PlatformView::Services => self.services_list.next(),
                PlatformView::PrivateComponents => {
                    self.private_packages_list.next();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.schedule_readme_load();
                }
                PlatformView::Registry => {
                    self.packages_list.next();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.schedule_readme_load();
                }
                PlatformView::Templates => {
                    self.templates_list.next();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                }
            },
            // Left/Right and h/l - switch between views
            KeyCode::Left | KeyCode::Char('h') => {
                self.platform_view = self.platform_view.previous();
                self.platform_desc_scroll_state = ScrollViewState::default();
                self.schedule_readme_load();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.platform_view = self.platform_view.next();
                self.platform_desc_scroll_state = ScrollViewState::default();
                self.schedule_readme_load();
            }
            // PageUp/PageDown - page scroll description
            KeyCode::PageUp => match self.platform_view {
                PlatformView::Services => {}
                PlatformView::PrivateComponents
                | PlatformView::Registry
                | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_page_up();
                }
            },
            KeyCode::PageDown => match self.platform_view {
                PlatformView::Services => {}
                PlatformView::PrivateComponents
                | PlatformView::Registry
                | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_page_down();
                }
            },
            // Home/g - go to first item
            KeyCode::Home | KeyCode::Char('g') => match self.platform_view {
                PlatformView::Services => self.services_list.select_first(),
                PlatformView::PrivateComponents => {
                    self.private_packages_list.select_first();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.schedule_readme_load();
                }
                PlatformView::Registry => {
                    self.packages_list.select_first();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.schedule_readme_load();
                }
                PlatformView::Templates => {
                    self.templates_list.select_first();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                }
            },
            // End/G - go to last item
            KeyCode::End | KeyCode::Char('G') => match self.platform_view {
                PlatformView::Services => self.services_list.select_last(),
                PlatformView::PrivateComponents => {
                    self.private_packages_list.select_last();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.schedule_readme_load();
                }
                PlatformView::Registry => {
                    self.packages_list.select_last();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.schedule_readme_load();
                }
                PlatformView::Templates => {
                    self.templates_list.select_last();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                }
            },
            // Number keys - jump to specific view
            KeyCode::Char('1') => {
                self.platform_view = PlatformView::Services;
                self.platform_desc_scroll_state = ScrollViewState::default();
            }
            KeyCode::Char('2') => {
                self.platform_view = PlatformView::PrivateComponents;
                self.platform_desc_scroll_state = ScrollViewState::default();
                self.spawn_readme_load_for_selected_private_package();
            }
            KeyCode::Char('3') => {
                self.platform_view = PlatformView::Registry;
                self.platform_desc_scroll_state = ScrollViewState::default();
                self.spawn_readme_load_for_selected_package();
            }
            KeyCode::Char('4') => {
                self.platform_view = PlatformView::Templates;
                self.platform_desc_scroll_state = ScrollViewState::default();
            }
            _ => {}
        }
    }

    /// Schedule a debounced README load (200ms after last navigation)
    pub(crate) fn schedule_readme_load(&mut self) {
        self.readme_debounce_deadline =
            Some(tokio::time::Instant::now() + std::time::Duration::from_millis(200));
    }
}
