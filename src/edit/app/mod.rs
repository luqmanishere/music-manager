use log::{debug, error, info, warn};
use tui_logger::TuiWidgetState;

use crate::edit::app::actions::Action;

use self::{actions::Actions, dir::DirListState, song::Song};

use super::{
    inputs::{key::Key, InputBuffer},
    io::IoEvent,
};

pub mod actions;
pub mod dir;
pub mod song;

#[derive(Debug, PartialEq, Eq)]
pub enum AppReturn {
    Exit,
    Continue,
}

pub struct App {
    /// Sender for IoEvent
    pub io_tx: tokio::sync::mpsc::Sender<IoEvent>,
    /// Available contextual actions
    actions: Actions,

    /// Input buffer
    pub input_buffer: InputBuffer,

    pub is_loading: bool, // States
    pub is_input: bool,
    // States
    pub current_app_widget: AppActiveWidgetState,
    pub previous_app_widget: AppActiveWidgetState,
    pub dirlist: DirListState,
    pub current_selected_song: Song,
    pub logs_state: TuiWidgetState,
}

impl App {
    /// Creates a new instance of App
    pub fn new(io_tx: tokio::sync::mpsc::Sender<IoEvent>) -> Self {
        let actions = vec![Action::Quit].into();

        //let state = AppState::initialized();
        Self {
            actions,
            io_tx,
            is_loading: false,
            current_selected_song: Default::default(),
            dirlist: DirListState::new(),
            logs_state: TuiWidgetState::new(),
            is_input: false,
            input_buffer: InputBuffer::new(),
            current_app_widget: AppActiveWidgetState::DirListing,
            previous_app_widget: AppActiveWidgetState::DirListing,
        }
    }

    /// Send actions to be executed in the I/O thread
    pub async fn dispatch(&mut self, action: IoEvent) {
        self.is_loading = true;

        if let Err(_e) = self.io_tx.send(action).await {
            self.is_loading = false;
            // log an error
        }
    }

    /// Actions to be executed in the UI thread
    pub async fn do_action(&mut self, key: Key) -> AppReturn {
        match self.is_input {
            false => {
                if let Some(action) = self.actions.find(key) {
                    debug!("Executing action: {}", action);
                    match action {
                        Action::Quit => AppReturn::Exit,
                        Action::LogToggleHideSelector => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::HideKey);
                            AppReturn::Continue
                        }
                        Action::LogToggleFocus => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::FocusKey);
                            AppReturn::Continue
                        }
                        Action::LogSelectPreviousTarget => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::UpKey);
                            AppReturn::Continue
                        }
                        Action::LogSelectNextTarget => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::DownKey);
                            AppReturn::Continue
                        }
                        Action::LogReduceShown => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::LeftKey);
                            AppReturn::Continue
                        }
                        Action::LogIncreaseShown => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::RightKey);
                            AppReturn::Continue
                        }
                        Action::LogDecreaseCapture => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::MinusKey);
                            AppReturn::Continue
                        }
                        Action::LogIncreaseCapture => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::PlusKey);
                            AppReturn::Continue
                        }
                        Action::LogPageUp => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::PrevPageKey);
                            AppReturn::Continue
                        }
                        Action::LogPageDown => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::NextPageKey);
                            AppReturn::Continue
                        }
                        Action::LogExitPageMode => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::EscapeKey);
                            AppReturn::Continue
                        }
                        Action::LogToggleHideTargets => {
                            self.logs_state
                                .transition(&tui_logger::TuiWidgetEvent::SpaceKey);
                            AppReturn::Continue
                        }
                        // End of LogWidget Actions
                        Action::SwitchToLogWidget => {
                            self.enter_log_viewer_widget();
                            AppReturn::Continue
                        }
                        Action::SwitchToPreviousWidget => {
                            debug!("Previous active widget: {:?}", self.previous_app_widget);
                            match self.previous_app_widget {
                                AppActiveWidgetState::DirListing => self.enter_dirlisting_widget(),
                                AppActiveWidgetState::MetadataEditor => {
                                    self.enter_metadata_editor_widget()
                                }
                                // go back to dirlistwidget as the default
                                _ => self.enter_dirlisting_widget(),
                            };
                            AppReturn::Continue
                        }
                        Action::SelectDown => {
                            match self.current_app_widget {
                                AppActiveWidgetState::DirListing => self.dirlist.next(),
                                AppActiveWidgetState::MetadataEditor => {
                                    self.current_selected_song.next()
                                }
                                _ => {}
                            }
                            AppReturn::Continue
                        }
                        Action::SelectUp => {
                            match self.current_app_widget {
                                AppActiveWidgetState::DirListing => self.dirlist.previous(),
                                AppActiveWidgetState::MetadataEditor => {
                                    self.current_selected_song.previous()
                                }
                                _ => {}
                            }
                            AppReturn::Continue
                        }
                        Action::Enter => {
                            match self.current_app_widget {
                                AppActiveWidgetState::DirListing => {
                                    self.enter_metadata_editor_widget()
                                }
                                AppActiveWidgetState::MetadataEditor => self.start_editing(),
                                _ => {}
                            }
                            AppReturn::Continue
                        }
                        Action::SaveTagsToFile => {
                            match self.current_selected_song.write_tag_changes() {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("Error saving tags to file: {}", e);
                                }
                            }
                            AppReturn::Continue
                        }
                        Action::SwitchToDirListWidget => {
                            self.enter_dirlisting_widget();
                            AppReturn::Continue
                        }
                    }
                } else {
                    warn!("No action was bound to key: {}", &key);
                    AppReturn::Continue
                }
            }

            true => {
                match key {
                    Key::Char(c) => self.input_buffer.push_char(c),
                    Key::Enter => {
                        self.stop_editing();
                        self.input_buffer.clear();
                    }
                    Key::Esc => {
                        // Exit the state and clear the buffer
                        self.is_input = false;
                        self.enter_metadata_editor_widget();
                    }
                    Key::Backspace => self.input_buffer.pop(),
                    _ => {}
                };
                AppReturn::Continue
            }
        }
    }

    pub async fn update_on_tick(&mut self) -> AppReturn {
        self.dirlist.poll();
        AppReturn::Continue
    }

    pub fn initialized(&mut self) {
        info!("Initialized.");
        self.enter_dirlisting_widget();
    }

    pub fn loaded(&mut self) {
        self.is_loading = false;
    }

    pub fn get_actions(&self) -> &Actions {
        &self.actions
    }

    fn set_actions(&mut self, actions: Vec<Action>) {
        self.actions = actions.into();
    }

    fn start_editing(&mut self) {
        self.is_input = true;
        self.previous_app_widget = self.current_app_widget;
        self.current_app_widget = AppActiveWidgetState::InputBar;
    }

    /// Handle actions after input
    fn stop_editing(&mut self) {
        self.is_input = false;
        match self.previous_app_widget {
            AppActiveWidgetState::DirListing => self.enter_dirlisting_widget(),
            AppActiveWidgetState::MetadataEditor => {
                self.current_selected_song
                    .edit(self.input_buffer.get_buffer_drain());
                self.enter_metadata_editor_widget();
            }
            // Don't do anything, as we don't want to return to this
            _ => {}
        };
    }

    /// Check if the given widget is selected
    pub fn is_selected(&self, widget: AppActiveWidgetState) -> bool {
        self.current_app_widget == widget
    }

    /// Execute upon entering DirList widget
    fn enter_dirlisting_widget(&mut self) {
        self.previous_app_widget = self.current_app_widget;
        self.current_app_widget = AppActiveWidgetState::DirListing;
        // Add dir list specific actions here
        self.set_actions(
            [
                Action::Quit,
                Action::SelectUp,
                Action::SelectDown,
                Action::Enter,
                Action::SwitchToLogWidget,
                Action::SwitchToPreviousWidget,
                Action::SwitchToDirListWidget,
            ]
            .into(),
        );
        info!("DirList widget is active");
    }

    /// Execute upon entering MetadataEditorWidget
    fn enter_metadata_editor_widget(&mut self) {
        if self.current_app_widget == AppActiveWidgetState::InputBar {
            self.previous_app_widget = AppActiveWidgetState::MetadataEditor
        } else {
            self.previous_app_widget = self.current_app_widget;
        }
        self.current_app_widget = AppActiveWidgetState::MetadataEditor;
        self.set_actions(
            [
                Action::Quit,
                Action::SelectUp,
                Action::SelectDown,
                Action::Enter,
                Action::SwitchToLogWidget,
                Action::SwitchToPreviousWidget,
                Action::SaveTagsToFile,
                Action::SwitchToDirListWidget,
            ]
            .into(),
        );
        if self.previous_app_widget == AppActiveWidgetState::DirListing {
            self.current_selected_song.initialized = false;
        }
        if !self.current_selected_song.is_initialized() {
            let path = self
                .dirlist
                .current_dir_file_paths
                .get(self.dirlist.state.selected().unwrap_or(0))
                .unwrap();
            self.current_selected_song = Song::read_music_file(path).unwrap();
        }
    }

    /// Execute upon entering LogViewerWidget
    fn enter_log_viewer_widget(&mut self) {
        self.previous_app_widget = self.current_app_widget;
        self.current_app_widget = AppActiveWidgetState::LogViewer;
        self.set_actions(
            [
                Action::LogDecreaseCapture,
                Action::LogExitPageMode,
                Action::LogIncreaseCapture,
                Action::LogIncreaseShown,
                Action::LogPageDown,
                Action::LogPageUp,
                Action::LogReduceShown,
                Action::LogSelectNextTarget,
                Action::LogSelectPreviousTarget,
                Action::LogToggleFocus,
                Action::LogToggleHideSelector,
                Action::LogToggleHideTargets,
                Action::SwitchToPreviousWidget,
            ]
            .into(),
        );
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AppActiveWidgetState {
    DirListing,
    MetadataEditor,
    LogViewer,
    InputBar,
}
