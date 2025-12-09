use ratatui::style::Color;

#[derive(Clone)]
pub struct Theme {
    pub name: &'static str,
    pub accent: Color,
    pub bg: Color,
    pub header_bg: Color,
    pub status_bg: Color,
    pub input_bg: Color,
    pub text: Color,
}

pub const THEMES: &[Theme] = &[
    Theme {
        name: "Dark",
        accent: Color::Rgb(92, 136, 255),
        bg: Color::Rgb(20, 20, 20),
        header_bg: Color::Rgb(40, 60, 110),
        status_bg: Color::Rgb(20, 20, 20),
        input_bg: Color::Rgb(30, 30, 30),
        text: Color::White,
    },
    Theme {
        name: "Midnight",
        accent: Color::Rgb(160, 120, 220),
        bg: Color::Rgb(15, 15, 20),
        header_bg: Color::Rgb(35, 20, 50),
        status_bg: Color::Rgb(15, 15, 20),
        input_bg: Color::Rgb(25, 25, 30),
        text: Color::White,
    },
    Theme {
        name: "Light",
        accent: Color::Rgb(0, 90, 180),
        bg: Color::Rgb(245, 245, 245),
        header_bg: Color::Rgb(245, 245, 250),
        status_bg: Color::Rgb(245, 245, 245),
        input_bg: Color::Rgb(255, 255, 255),
        text: Color::Rgb(20, 20, 20),
    },
];

