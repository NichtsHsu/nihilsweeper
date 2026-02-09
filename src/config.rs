use crate::base::board;
use log::{debug, error, info};
use pastey::paste;

macro_rules! update_config {
    (
        #[$meta:meta]
        $vis:vis struct $name:ident {
            $($field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        #[$meta]
        $vis struct $name {
            $( $field_vis $field_name : $field_type ),*
        }

        paste! {
            #[derive(Debug, Clone, Default)]
            $vis struct [<$name Update>] {
                updated: bool,
                $( $field_vis $field_name: Option<$field_type> ),*
            }

            impl [<$name Update>] {
                $(
                    $vis fn $field_name(&mut self, value: $field_type) -> &mut Self {
                        self.$field_name = Some(value);
                        self.updated = true;
                        self
                    }
                )*

                $vis fn is_updated(&self) -> bool {
                    self.updated
                }

                $vis fn apply_to(self, config: &mut $name) {
                    $(
                        if let Some(value) = self.$field_name {
                            config.$field_name = value;
                        }
                    )*
                }
            }
        }
    };
}

update_config! {
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct GlobalConfig {
        pub skin: String,
        pub cell_size: u32,
        pub board: [usize; 3], // width, height, mines
        pub chord_mode: board::ChordMode,
    }
}

impl GlobalConfig {
    pub fn save(&self) -> crate::error::Result<()> {
        let config_path = dirs::config_dir()
            .unwrap_or(".".into())
            .join(env!("CARGO_CRATE_NAME"))
            .join("config.toml");

        debug!("Saving config to {:?}", config_path);
        std::fs::create_dir_all(config_path.parent().unwrap())
            .inspect_err(|e| error!("Failed to create config directory: {e}"))?;
        let config_data = toml::to_string(self).inspect_err(|e| error!("Failed to serialize config: {e}"))?;
        std::fs::write(&config_path, config_data).inspect_err(|e| error!("Failed to write config file: {e}"))?;
        info!("Configuration saved successfully");
        Ok(())
    }

    pub fn load() -> crate::error::Result<Self> {
        let config_path = dirs::config_dir()
            .unwrap_or(".".into())
            .join(env!("CARGO_CRATE_NAME"))
            .join("config.toml");

        debug!("Loading config from {:?}", config_path);
        let config_data =
            std::fs::read_to_string(config_path).inspect_err(|e| error!("Failed to read config file: {e}"))?;
        let config = toml::from_str(&config_data).inspect_err(|e| error!("Failed to deserialize config: {e}"))?;
        info!("Configuration loaded successfully");
        Ok(config)
    }
}
