mod bottles;
mod heroic;
mod steam;

#[cfg(feature = "lutris")]
mod lutris;

use crate::model::{Session, Source};
use bottles::BottlesIndex;
use heroic::HeroicIndex;
use steam::SteamIndex;

#[cfg(feature = "lutris")]
use lutris::LutrisIndex;

/// Launcher metadata applied after classification.
pub struct Enricher {
    steam: SteamIndex,
    #[cfg(feature = "lutris")]
    lutris: LutrisIndex,
    heroic: HeroicIndex,
    bottles: BottlesIndex,
}

impl Enricher {
    pub fn load() -> Self {
        Self {
            steam: SteamIndex::load(),
            #[cfg(feature = "lutris")]
            lutris: LutrisIndex::load(),
            heroic: HeroicIndex::load(),
            bottles: BottlesIndex::load(),
        }
    }

    pub fn apply(&self, sessions: &mut [Session]) {
        for session in sessions.iter_mut() {
            if let Some(appid) = session.steam_app_id {
                if let Some(name) = self.steam.name_for(appid) {
                    session.name = name;
                    session.source = Source::Steam;
                }
            }

            #[cfg(feature = "lutris")]
            if let Some(ref prefix) = session.prefix {
                if let Some(name) = self.lutris.name_for_prefix(prefix) {
                    if session.source == Source::Lutris || session.source == Source::Wine {
                        session.name = name;
                        session.source = Source::Lutris;
                    }
                }
            }

            if let Some(ref prefix) = session.prefix {
                if let Some(name) = self.heroic.name_for_prefix(prefix) {
                    session.name = name;
                    if session.source == Source::Wine {
                        session.source = Source::Heroic;
                    }
                }
                if let Some(name) = self.bottles.name_for_prefix(prefix) {
                    session.name = name;
                    session.source = Source::Bottles;
                }
            }

            if session.source == Source::Bottles {
                session.notes.push(
                    "If this bottle uses a Flatpak sandbox, some PIDs may be hidden from host /proc"
                        .into(),
                );
                if self
                    .bottles
                    .opaque_hints()
                    .iter()
                    .any(|h| h == &session.name)
                {
                    session.opaque = true;
                }
            }
        }
    }
}
