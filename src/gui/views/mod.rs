use iced::{
    Size, Task,
    widget::svg,
    window::{self, Id},
};

pub mod extra;
pub mod main;

pub fn open(window_type: WindowType) -> (Id, Task<Id>) {
    let size = match window_type {
        WindowType::Main => Size::new(500.0, main::WINDOW_HEIGHT),
        WindowType::Extra => Size::new(400.0, 300.0),
    };

    let settings = window::Settings {
        size,
        resizable: false,
        icon: window::icon::from_file_data(include_bytes!("../../../res/icon.png"), None).ok(),
        ..window::Settings::default()
    };
    window::open(settings)
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WindowType {
    Main,
    Extra,
}

pub struct Icons {
    search: svg::Handle,
    import: svg::Handle,
    export: svg::Handle,
    _globe: svg::Handle,
    plus: svg::Handle,
}

impl Icons {
    pub fn new() -> Self {
        Self {
            search: svg::Handle::from_memory(
                include_bytes!("../../../res/phosphor/magnifying-glass.svg").as_slice(),
            ),
            import: svg::Handle::from_memory(
                include_bytes!("../../../res/phosphor/arrow-square-in.svg").as_slice(),
            ),
            export: svg::Handle::from_memory(
                include_bytes!("../../../res/phosphor/arrow-square-out.svg").as_slice(),
            ),
            _globe: svg::Handle::from_memory(
                include_bytes!("../../../res/phosphor/globe.svg").as_slice(),
            ),
            plus: svg::Handle::from_memory(
                include_bytes!("../../../res/phosphor/plus.svg").as_slice(),
            ),
        }
    }
}
