use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub user_agent: String,
    pub screen_width: u32,
    pub screen_height: u32,
    pub hardware_concurrency: u32, // Must be realistic: 4, 8, or 16
    pub device_memory: u32,        // 4 or 8
    pub platform: String,
    pub color_depth: u32, // 24 or 32
}

pub struct PersonaPool {
    cache: RwLock<HashMap<Uuid, Persona>>,
}

impl Default for PersonaPool {
    fn default() -> Self {
        Self::new()
    }
}

impl PersonaPool {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Assigns or retrieves an existing persona bound to a session UUID.
    /// Uses a deterministically seeded RNG per UUID to ensure identical
    /// sessions always produce identical anti-detect fingerprints.
    pub fn get_or_create(&self, session_id: Uuid) -> Persona {
        // 1. Check cache first
        if let Some(persona) = self.cache.read().unwrap().get(&session_id) {
            return persona.clone();
        }

        // 2. Generate deterministic persona from the UUID bytes
        let mut seed = [0u8; 32];
        let bytes = session_id.into_bytes();
        seed[..16].copy_from_slice(&bytes);
        seed[16..].copy_from_slice(&bytes); // duplicate to fill 32 bytes

        let mut rng = StdRng::from_seed(seed);

        let persona = Self::generate_profile(&mut rng);
        self.cache
            .write()
            .unwrap()
            .insert(session_id, persona.clone());

        persona
    }

    fn generate_profile(rng: &mut StdRng) -> Persona {
        // D-13 & D-14: Chrome 130 realistic base profiles
        let platforms = ["Win32", "MacIntel", "Linux x86_64"];
        let user_agents = [
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
        ];

        let idx = rng.gen_range(0..3);
        let platform = platforms[idx].to_string();
        let user_agent = user_agents[idx].to_string();

        let hardware_opts = [4, 8, 16];
        let hardware_concurrency = hardware_opts[rng.gen_range(0..3)];

        let memory_opts = [4, 8];
        let device_memory = memory_opts[rng.gen_range(0..2)];

        let color_opts = [24, 32];
        let color_depth = color_opts[rng.gen_range(0..2)];

        // Realistic resolutions
        let resolutions = [(1920, 1080), (1366, 768), (1440, 900), (2560, 1440)];
        let (screen_width, screen_height) = resolutions[rng.gen_range(0..4)];

        Persona {
            user_agent,
            screen_width,
            screen_height,
            hardware_concurrency,
            device_memory,
            platform,
            color_depth,
        }
    }
}
