mod audio;
mod ui;
mod utils;

use ui::app::VolumeApp;

fn main() -> iced::Result {
    iced::application(VolumeApp::new, VolumeApp::update, VolumeApp::view)
        .subscription(VolumeApp::subscription)
        .run()
}
