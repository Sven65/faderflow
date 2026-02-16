use iced::window::icon;


pub fn load_icon() -> icon::Icon {
    let icon_bytes = include_bytes!("../../res/icon.png");
    let image = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon")
        .to_rgba8();

    let (width, height) = image.dimensions();
    let rgba = image.into_raw();

    icon::from_rgba(rgba, width, height)
        .expect("Failed to create icon")
}