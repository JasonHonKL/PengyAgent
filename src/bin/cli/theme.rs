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
    // Original themes
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
    // Dracula Theme
    Theme {
        name: "Dracula",
        accent: Color::Rgb(189, 147, 249),
        bg: Color::Rgb(40, 42, 54),
        header_bg: Color::Rgb(68, 71, 90),
        status_bg: Color::Rgb(40, 42, 54),
        input_bg: Color::Rgb(50, 52, 64),
        text: Color::Rgb(248, 248, 242),
    },
    // Nord Theme
    Theme {
        name: "Nord",
        accent: Color::Rgb(136, 192, 208),
        bg: Color::Rgb(46, 52, 64),
        header_bg: Color::Rgb(59, 66, 82),
        status_bg: Color::Rgb(46, 52, 64),
        input_bg: Color::Rgb(59, 66, 82),
        text: Color::Rgb(236, 239, 244),
    },
    // Gruvbox Dark
    Theme {
        name: "Gruvbox Dark",
        accent: Color::Rgb(251, 73, 52),
        bg: Color::Rgb(40, 40, 40),
        header_bg: Color::Rgb(60, 56, 54),
        status_bg: Color::Rgb(40, 40, 40),
        input_bg: Color::Rgb(50, 48, 47),
        text: Color::Rgb(235, 219, 178),
    },
    // Gruvbox Light
    Theme {
        name: "Gruvbox Light",
        accent: Color::Rgb(204, 36, 29),
        bg: Color::Rgb(251, 241, 199),
        header_bg: Color::Rgb(235, 219, 178),
        status_bg: Color::Rgb(251, 241, 199),
        input_bg: Color::Rgb(255, 255, 255),
        text: Color::Rgb(60, 56, 54),
    },
    // Monokai
    Theme {
        name: "Monokai",
        accent: Color::Rgb(249, 38, 114),
        bg: Color::Rgb(39, 40, 34),
        header_bg: Color::Rgb(55, 56, 48),
        status_bg: Color::Rgb(39, 40, 34),
        input_bg: Color::Rgb(50, 51, 43),
        text: Color::Rgb(248, 248, 242),
    },
    // One Dark Pro
    Theme {
        name: "One Dark Pro",
        accent: Color::Rgb(97, 175, 239),
        bg: Color::Rgb(40, 44, 52),
        header_bg: Color::Rgb(53, 59, 69),
        status_bg: Color::Rgb(40, 44, 52),
        input_bg: Color::Rgb(50, 54, 62),
        text: Color::Rgb(171, 178, 191),
    },
    // Solarized Dark
    Theme {
        name: "Solarized Dark",
        accent: Color::Rgb(38, 139, 210),
        bg: Color::Rgb(0, 43, 54),
        header_bg: Color::Rgb(7, 54, 66),
        status_bg: Color::Rgb(0, 43, 54),
        input_bg: Color::Rgb(7, 54, 66),
        text: Color::Rgb(131, 148, 150),
    },
    // Solarized Light
    Theme {
        name: "Solarized Light",
        accent: Color::Rgb(38, 139, 210),
        bg: Color::Rgb(253, 246, 227),
        header_bg: Color::Rgb(238, 232, 213),
        status_bg: Color::Rgb(253, 246, 227),
        input_bg: Color::Rgb(255, 255, 255),
        text: Color::Rgb(101, 123, 131),
    },
    // Catppuccin Mocha
    Theme {
        name: "Catppuccin Mocha",
        accent: Color::Rgb(137, 180, 250),
        bg: Color::Rgb(30, 30, 46),
        header_bg: Color::Rgb(49, 50, 68),
        status_bg: Color::Rgb(30, 30, 46),
        input_bg: Color::Rgb(49, 50, 68),
        text: Color::Rgb(205, 214, 244),
    },
    // Catppuccin Latte
    Theme {
        name: "Catppuccin Latte",
        accent: Color::Rgb(30, 102, 245),
        bg: Color::Rgb(239, 241, 245),
        header_bg: Color::Rgb(220, 224, 232),
        status_bg: Color::Rgb(239, 241, 245),
        input_bg: Color::Rgb(255, 255, 255),
        text: Color::Rgb(76, 79, 105),
    },
    // Tokyo Night
    Theme {
        name: "Tokyo Night",
        accent: Color::Rgb(125, 207, 255),
        bg: Color::Rgb(26, 27, 38),
        header_bg: Color::Rgb(36, 40, 59),
        status_bg: Color::Rgb(26, 27, 38),
        input_bg: Color::Rgb(36, 40, 59),
        text: Color::Rgb(192, 202, 245),
    },
    // Material Dark
    Theme {
        name: "Material Dark",
        accent: Color::Rgb(103, 159, 181),
        bg: Color::Rgb(29, 29, 29),
        header_bg: Color::Rgb(38, 50, 56),
        status_bg: Color::Rgb(29, 29, 29),
        input_bg: Color::Rgb(40, 40, 40),
        text: Color::Rgb(238, 255, 255),
    },
    // Material Light
    Theme {
        name: "Material Light",
        accent: Color::Rgb(0, 122, 204),
        bg: Color::Rgb(250, 250, 250),
        header_bg: Color::Rgb(224, 224, 224),
        status_bg: Color::Rgb(250, 250, 250),
        input_bg: Color::Rgb(255, 255, 255),
        text: Color::Rgb(33, 33, 33),
    },
    // GitHub Dark
    Theme {
        name: "GitHub Dark",
        accent: Color::Rgb(79, 140, 201),
        bg: Color::Rgb(13, 17, 23),
        header_bg: Color::Rgb(22, 27, 34),
        status_bg: Color::Rgb(13, 17, 23),
        input_bg: Color::Rgb(22, 27, 34),
        text: Color::Rgb(201, 209, 217),
    },
    // GitHub Light
    Theme {
        name: "GitHub Light",
        accent: Color::Rgb(9, 105, 218),
        bg: Color::Rgb(255, 255, 255),
        header_bg: Color::Rgb(246, 248, 250),
        status_bg: Color::Rgb(255, 255, 255),
        input_bg: Color::Rgb(255, 255, 255),
        text: Color::Rgb(36, 41, 47),
    },
    // Ayu Dark
    Theme {
        name: "Ayu Dark",
        accent: Color::Rgb(255, 204, 102),
        bg: Color::Rgb(15, 20, 25),
        header_bg: Color::Rgb(23, 31, 40),
        status_bg: Color::Rgb(15, 20, 25),
        input_bg: Color::Rgb(23, 31, 40),
        text: Color::Rgb(230, 230, 230),
    },
    // Ayu Mirage
    Theme {
        name: "Ayu Mirage",
        accent: Color::Rgb(255, 204, 102),
        bg: Color::Rgb(31, 36, 48),
        header_bg: Color::Rgb(39, 46, 60),
        status_bg: Color::Rgb(31, 36, 48),
        input_bg: Color::Rgb(39, 46, 60),
        text: Color::Rgb(203, 204, 198),
    },
    // Ayu Light
    Theme {
        name: "Ayu Light",
        accent: Color::Rgb(255, 153, 0),
        bg: Color::Rgb(251, 252, 253),
        header_bg: Color::Rgb(245, 247, 250),
        status_bg: Color::Rgb(251, 252, 253),
        input_bg: Color::Rgb(255, 255, 255),
        text: Color::Rgb(87, 96, 111),
    },
    // Tomorrow Night
    Theme {
        name: "Tomorrow Night",
        accent: Color::Rgb(129, 162, 190),
        bg: Color::Rgb(29, 31, 33),
        header_bg: Color::Rgb(40, 42, 46),
        status_bg: Color::Rgb(29, 31, 33),
        input_bg: Color::Rgb(40, 42, 46),
        text: Color::Rgb(197, 200, 198),
    },
    // Tomorrow Night Eighties
    Theme {
        name: "Tomorrow Night 80s",
        accent: Color::Rgb(102, 153, 204),
        bg: Color::Rgb(45, 45, 45),
        header_bg: Color::Rgb(57, 57, 57),
        status_bg: Color::Rgb(45, 45, 45),
        input_bg: Color::Rgb(57, 57, 57),
        text: Color::Rgb(204, 204, 204),
    },
    // PaperColor Dark
    Theme {
        name: "PaperColor Dark",
        accent: Color::Rgb(87, 166, 74),
        bg: Color::Rgb(28, 28, 28),
        header_bg: Color::Rgb(46, 46, 46),
        status_bg: Color::Rgb(28, 28, 28),
        input_bg: Color::Rgb(46, 46, 46),
        text: Color::Rgb(212, 212, 212),
    },
    // PaperColor Light
    Theme {
        name: "PaperColor Light",
        accent: Color::Rgb(87, 166, 74),
        bg: Color::Rgb(238, 238, 238),
        header_bg: Color::Rgb(250, 250, 250),
        status_bg: Color::Rgb(238, 238, 238),
        input_bg: Color::Rgb(255, 255, 255),
        text: Color::Rgb(68, 68, 68),
    },
    // Oceanic Next
    Theme {
        name: "Oceanic Next",
        accent: Color::Rgb(102, 217, 239),
        bg: Color::Rgb(23, 25, 35),
        header_bg: Color::Rgb(34, 38, 52),
        status_bg: Color::Rgb(23, 25, 35),
        input_bg: Color::Rgb(34, 38, 52),
        text: Color::Rgb(192, 197, 206),
    },
    // Palenight
    Theme {
        name: "Palenight",
        accent: Color::Rgb(130, 170, 255),
        bg: Color::Rgb(41, 45, 62),
        header_bg: Color::Rgb(52, 57, 77),
        status_bg: Color::Rgb(41, 45, 62),
        input_bg: Color::Rgb(52, 57, 77),
        text: Color::Rgb(200, 203, 210),
    },
    // Snazzy
    Theme {
        name: "Snazzy",
        accent: Color::Rgb(255, 184, 108),
        bg: Color::Rgb(40, 42, 54),
        header_bg: Color::Rgb(68, 71, 90),
        status_bg: Color::Rgb(40, 42, 54),
        input_bg: Color::Rgb(68, 71, 90),
        text: Color::Rgb(248, 248, 242),
    },
    // Synthwave '84
    Theme {
        name: "Synthwave '84",
        accent: Color::Rgb(255, 119, 198),
        bg: Color::Rgb(36, 27, 52),
        header_bg: Color::Rgb(58, 44, 82),
        status_bg: Color::Rgb(36, 27, 52),
        input_bg: Color::Rgb(58, 44, 82),
        text: Color::Rgb(255, 255, 255),
    },
    // Cobalt
    Theme {
        name: "Cobalt",
        accent: Color::Rgb(255, 204, 0),
        bg: Color::Rgb(0, 34, 68),
        header_bg: Color::Rgb(0, 51, 102),
        status_bg: Color::Rgb(0, 34, 68),
        input_bg: Color::Rgb(0, 51, 102),
        text: Color::Rgb(255, 255, 255),
    },
    // Zenburn
    Theme {
        name: "Zenburn",
        accent: Color::Rgb(220, 220, 204),
        bg: Color::Rgb(63, 63, 63),
        header_bg: Color::Rgb(77, 77, 77),
        status_bg: Color::Rgb(63, 63, 63),
        input_bg: Color::Rgb(77, 77, 77),
        text: Color::Rgb(220, 220, 204),
    },
    // Base16 Dark
    Theme {
        name: "Base16 Dark",
        accent: Color::Rgb(184, 187, 38),
        bg: Color::Rgb(21, 21, 21),
        header_bg: Color::Rgb(40, 40, 40),
        status_bg: Color::Rgb(21, 21, 21),
        input_bg: Color::Rgb(40, 40, 40),
        text: Color::Rgb(212, 212, 212),
    },
    // Base16 Light
    Theme {
        name: "Base16 Light",
        accent: Color::Rgb(136, 138, 133),
        bg: Color::Rgb(245, 245, 245),
        header_bg: Color::Rgb(230, 230, 230),
        status_bg: Color::Rgb(245, 245, 245),
        input_bg: Color::Rgb(255, 255, 255),
        text: Color::Rgb(32, 32, 32),
    },
    // Spacegray
    Theme {
        name: "Spacegray",
        accent: Color::Rgb(255, 255, 255),
        bg: Color::Rgb(34, 34, 34),
        header_bg: Color::Rgb(50, 50, 50),
        status_bg: Color::Rgb(34, 34, 34),
        input_bg: Color::Rgb(50, 50, 50),
        text: Color::Rgb(255, 255, 255),
    },
    // Flatland
    Theme {
        name: "Flatland",
        accent: Color::Rgb(106, 159, 181),
        bg: Color::Rgb(39, 40, 34),
        header_bg: Color::Rgb(50, 52, 44),
        status_bg: Color::Rgb(39, 40, 34),
        input_bg: Color::Rgb(50, 52, 44),
        text: Color::Rgb(248, 248, 242),
    },
    // Wombat
    Theme {
        name: "Wombat",
        accent: Color::Rgb(184, 187, 38),
        bg: Color::Rgb(22, 22, 22),
        header_bg: Color::Rgb(40, 40, 40),
        status_bg: Color::Rgb(22, 22, 22),
        input_bg: Color::Rgb(40, 40, 40),
        text: Color::Rgb(212, 212, 212),
    },
    // Molokai
    Theme {
        name: "Molokai",
        accent: Color::Rgb(174, 129, 255),
        bg: Color::Rgb(27, 27, 27),
        header_bg: Color::Rgb(46, 46, 46),
        status_bg: Color::Rgb(27, 27, 27),
        input_bg: Color::Rgb(46, 46, 46),
        text: Color::Rgb(248, 248, 242),
    },
    // Jellybeans
    Theme {
        name: "Jellybeans",
        accent: Color::Rgb(255, 204, 102),
        bg: Color::Rgb(21, 21, 21),
        header_bg: Color::Rgb(40, 40, 40),
        status_bg: Color::Rgb(21, 21, 21),
        input_bg: Color::Rgb(40, 40, 40),
        text: Color::Rgb(212, 212, 212),
    },
    // Desert
    Theme {
        name: "Desert",
        accent: Color::Rgb(255, 140, 0),
        bg: Color::Rgb(51, 51, 51),
        header_bg: Color::Rgb(68, 68, 68),
        status_bg: Color::Rgb(51, 51, 51),
        input_bg: Color::Rgb(68, 68, 68),
        text: Color::Rgb(255, 255, 255),
    },
    // Peacock
    Theme {
        name: "Peacock",
        accent: Color::Rgb(0, 191, 255),
        bg: Color::Rgb(33, 33, 33),
        header_bg: Color::Rgb(50, 50, 50),
        status_bg: Color::Rgb(33, 33, 33),
        input_bg: Color::Rgb(50, 50, 50),
        text: Color::Rgb(255, 255, 255),
    },
    // Twilight
    Theme {
        name: "Twilight",
        accent: Color::Rgb(205, 168, 105),
        bg: Color::Rgb(20, 20, 20),
        header_bg: Color::Rgb(40, 40, 40),
        status_bg: Color::Rgb(20, 20, 20),
        input_bg: Color::Rgb(40, 40, 40),
        text: Color::Rgb(255, 255, 255),
    },
    // Vibrant Ink
    Theme {
        name: "Vibrant Ink",
        accent: Color::Rgb(255, 255, 0),
        bg: Color::Rgb(0, 0, 0),
        header_bg: Color::Rgb(20, 20, 20),
        status_bg: Color::Rgb(0, 0, 0),
        input_bg: Color::Rgb(20, 20, 20),
        text: Color::Rgb(255, 255, 255),
    },
    // Pastel Dark
    Theme {
        name: "Pastel Dark",
        accent: Color::Rgb(174, 129, 255),
        bg: Color::Rgb(30, 30, 30),
        header_bg: Color::Rgb(50, 50, 50),
        status_bg: Color::Rgb(30, 30, 30),
        input_bg: Color::Rgb(50, 50, 50),
        text: Color::Rgb(255, 255, 255),
    },
    // Pastel Light
    Theme {
        name: "Pastel Light",
        accent: Color::Rgb(174, 129, 255),
        bg: Color::Rgb(255, 255, 255),
        header_bg: Color::Rgb(240, 240, 240),
        status_bg: Color::Rgb(255, 255, 255),
        input_bg: Color::Rgb(255, 255, 255),
        text: Color::Rgb(0, 0, 0),
    },
    // Blueberry
    Theme {
        name: "Blueberry",
        accent: Color::Rgb(135, 206, 250),
        bg: Color::Rgb(30, 30, 50),
        header_bg: Color::Rgb(50, 50, 70),
        status_bg: Color::Rgb(30, 30, 50),
        input_bg: Color::Rgb(50, 50, 70),
        text: Color::Rgb(255, 255, 255),
    },
    // Cherry
    Theme {
        name: "Cherry",
        accent: Color::Rgb(255, 105, 180),
        bg: Color::Rgb(40, 20, 30),
        header_bg: Color::Rgb(60, 30, 45),
        status_bg: Color::Rgb(40, 20, 30),
        input_bg: Color::Rgb(60, 30, 45),
        text: Color::Rgb(255, 255, 255),
    },
    // Forest
    Theme {
        name: "Forest",
        accent: Color::Rgb(144, 238, 144),
        bg: Color::Rgb(20, 30, 20),
        header_bg: Color::Rgb(30, 45, 30),
        status_bg: Color::Rgb(20, 30, 20),
        input_bg: Color::Rgb(30, 45, 30),
        text: Color::Rgb(255, 255, 255),
    },
    // Sunset
    Theme {
        name: "Sunset",
        accent: Color::Rgb(255, 165, 0),
        bg: Color::Rgb(40, 30, 20),
        header_bg: Color::Rgb(60, 45, 30),
        status_bg: Color::Rgb(40, 30, 20),
        input_bg: Color::Rgb(60, 45, 30),
        text: Color::Rgb(255, 255, 255),
    },
    // Ocean
    Theme {
        name: "Ocean",
        accent: Color::Rgb(0, 191, 255),
        bg: Color::Rgb(20, 30, 40),
        header_bg: Color::Rgb(30, 45, 60),
        status_bg: Color::Rgb(20, 30, 40),
        input_bg: Color::Rgb(30, 45, 60),
        text: Color::Rgb(255, 255, 255),
    },
    // Lavender
    Theme {
        name: "Lavender",
        accent: Color::Rgb(230, 230, 250),
        bg: Color::Rgb(40, 30, 50),
        header_bg: Color::Rgb(60, 45, 70),
        status_bg: Color::Rgb(40, 30, 50),
        input_bg: Color::Rgb(60, 45, 70),
        text: Color::Rgb(255, 255, 255),
    },
    // Mint
    Theme {
        name: "Mint",
        accent: Color::Rgb(152, 251, 152),
        bg: Color::Rgb(20, 40, 30),
        header_bg: Color::Rgb(30, 60, 45),
        status_bg: Color::Rgb(20, 40, 30),
        input_bg: Color::Rgb(30, 60, 45),
        text: Color::Rgb(255, 255, 255),
    },
    // Rose
    Theme {
        name: "Rose",
        accent: Color::Rgb(255, 192, 203),
        bg: Color::Rgb(50, 30, 35),
        header_bg: Color::Rgb(70, 40, 50),
        status_bg: Color::Rgb(50, 30, 35),
        input_bg: Color::Rgb(70, 40, 50),
        text: Color::Rgb(255, 255, 255),
    },
    // Amber
    Theme {
        name: "Amber",
        accent: Color::Rgb(255, 193, 7),
        bg: Color::Rgb(40, 30, 20),
        header_bg: Color::Rgb(60, 45, 30),
        status_bg: Color::Rgb(40, 30, 20),
        input_bg: Color::Rgb(60, 45, 30),
        text: Color::Rgb(255, 255, 255),
    },
    // Emerald
    Theme {
        name: "Emerald",
        accent: Color::Rgb(80, 200, 120),
        bg: Color::Rgb(20, 40, 25),
        header_bg: Color::Rgb(30, 60, 40),
        status_bg: Color::Rgb(20, 40, 25),
        input_bg: Color::Rgb(30, 60, 40),
        text: Color::Rgb(255, 255, 255),
    },
    // Slate
    Theme {
        name: "Slate",
        accent: Color::Rgb(112, 128, 144),
        bg: Color::Rgb(47, 79, 79),
        header_bg: Color::Rgb(70, 100, 100),
        status_bg: Color::Rgb(47, 79, 79),
        input_bg: Color::Rgb(70, 100, 100),
        text: Color::Rgb(255, 255, 255),
    },
    // Crimson
    Theme {
        name: "Crimson",
        accent: Color::Rgb(220, 20, 60),
        bg: Color::Rgb(40, 15, 20),
        header_bg: Color::Rgb(60, 25, 30),
        status_bg: Color::Rgb(40, 15, 20),
        input_bg: Color::Rgb(60, 25, 30),
        text: Color::Rgb(255, 255, 255),
    },
    // Indigo
    Theme {
        name: "Indigo",
        accent: Color::Rgb(75, 0, 130),
        bg: Color::Rgb(25, 25, 40),
        header_bg: Color::Rgb(40, 40, 60),
        status_bg: Color::Rgb(25, 25, 40),
        input_bg: Color::Rgb(40, 40, 60),
        text: Color::Rgb(255, 255, 255),
    },
    // Teal
    Theme {
        name: "Teal",
        accent: Color::Rgb(0, 128, 128),
        bg: Color::Rgb(20, 40, 40),
        header_bg: Color::Rgb(30, 60, 60),
        status_bg: Color::Rgb(20, 40, 40),
        input_bg: Color::Rgb(30, 60, 60),
        text: Color::Rgb(255, 255, 255),
    },
    // Purple
    Theme {
        name: "Purple",
        accent: Color::Rgb(186, 85, 211),
        bg: Color::Rgb(40, 20, 50),
        header_bg: Color::Rgb(60, 30, 70),
        status_bg: Color::Rgb(40, 20, 50),
        input_bg: Color::Rgb(60, 30, 70),
        text: Color::Rgb(255, 255, 255),
    },
    // Orange
    Theme {
        name: "Orange",
        accent: Color::Rgb(255, 140, 0),
        bg: Color::Rgb(50, 30, 20),
        header_bg: Color::Rgb(70, 45, 30),
        status_bg: Color::Rgb(50, 30, 20),
        input_bg: Color::Rgb(70, 45, 30),
        text: Color::Rgb(255, 255, 255),
    },
    // Cyan
    Theme {
        name: "Cyan",
        accent: Color::Rgb(0, 255, 255),
        bg: Color::Rgb(20, 40, 50),
        header_bg: Color::Rgb(30, 60, 70),
        status_bg: Color::Rgb(20, 40, 50),
        input_bg: Color::Rgb(30, 60, 70),
        text: Color::Rgb(255, 255, 255),
    },
    // Lime
    Theme {
        name: "Lime",
        accent: Color::Rgb(50, 205, 50),
        bg: Color::Rgb(30, 50, 20),
        header_bg: Color::Rgb(45, 75, 30),
        status_bg: Color::Rgb(30, 50, 20),
        input_bg: Color::Rgb(45, 75, 30),
        text: Color::Rgb(255, 255, 255),
    },
    // Pink
    Theme {
        name: "Pink",
        accent: Color::Rgb(255, 20, 147),
        bg: Color::Rgb(50, 25, 40),
        header_bg: Color::Rgb(70, 35, 55),
        status_bg: Color::Rgb(50, 25, 40),
        input_bg: Color::Rgb(70, 35, 55),
        text: Color::Rgb(255, 255, 255),
    },
    // Gold
    Theme {
        name: "Gold",
        accent: Color::Rgb(255, 215, 0),
        bg: Color::Rgb(50, 40, 20),
        header_bg: Color::Rgb(70, 60, 30),
        status_bg: Color::Rgb(50, 40, 20),
        input_bg: Color::Rgb(70, 60, 30),
        text: Color::Rgb(255, 255, 255),
    },
    // Silver
    Theme {
        name: "Silver",
        accent: Color::Rgb(192, 192, 192),
        bg: Color::Rgb(40, 40, 40),
        header_bg: Color::Rgb(60, 60, 60),
        status_bg: Color::Rgb(40, 40, 40),
        input_bg: Color::Rgb(60, 60, 60),
        text: Color::Rgb(255, 255, 255),
    },
];
