use ratatui::style::{Color, Modifier, Style};
use winetop_core::Source;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    Default,
    Dim,
    HighContrast,
}

impl ThemeId {
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Dim,
            Self::Dim => Self::HighContrast,
            Self::HighContrast => Self::Default,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Dim => "dim",
            Self::HighContrast => "hc",
        }
    }
}

pub struct Theme {
    pub id: ThemeId,
}

impl Theme {
    pub fn new(id: ThemeId) -> Self {
        Self { id }
    }

    pub fn header(&self) -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn footer(&self) -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn selected(&self) -> Style {
        match self.id {
            ThemeId::HighContrast => Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            _ => Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn normal(&self) -> Style {
        Style::default().fg(Color::Gray)
    }

    pub fn source(&self, source: Source) -> Style {
        let fg = match (self.id, source) {
            (_, Source::Steam) => Color::Blue,
            (_, Source::Lutris) => Color::Magenta,
            (_, Source::Heroic) => Color::Yellow,
            (_, Source::Bottles) => Color::Green,
            (_, Source::Wine) => Color::Cyan,
            (_, Source::Unknown) => Color::Gray,
        };
        Style::default().fg(fg)
    }

    pub fn warn(&self) -> Style {
        Style::default().fg(Color::Red)
    }

    pub fn spark(&self) -> Style {
        Style::default().fg(Color::Cyan)
    }
}
