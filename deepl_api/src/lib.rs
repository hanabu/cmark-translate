pub struct Deepl {
    config: DeeplConfig,
}

impl Deepl {
    // New DeepL instance with .deepl ~/.deepl
    pub fn new() -> std::io::Result<Self> {
        let deepl_config = DeeplConfig::new()?;

        Ok(Self {
            config: deepl_config,
        })
    }

    pub async fn translate(
        &self,
        from_lang: Language,
        to_lang: Language,
        body: &str,
    ) -> reqwest::Result<String> {
        let client = reqwest::Client::new();

        // Make DeepL API request
        let resp = client
            .post(self.config.endpoint())
            .header(
                "authorization",
                format!("DeepL-Auth-Key {}", self.config.deepl_api_key),
            )
            .form(&[
                ("source_lang", from_lang.as_langcode()),
                ("target_lang", to_lang.as_langcode()),
                ("formality", "prefer_less"),
                ("text", body),
            ])
            .send()
            .await?;

        let mut deepl_resp = resp.json::<DeeplResponse>().await?;
        if 0 < deepl_resp.translations.len() {
            Ok(deepl_resp.translations.swap_remove(0).text)
        } else {
            // Empty response
            Ok(String::new())
        }
    }
}

#[derive(serde::Deserialize)]
pub enum Language {
    De,
    Es,
    En,
    Fr,
    It,
    Ja,
    Nl,
    Pt,
    Ru,
}

impl Language {
    fn as_langcode(&self) -> &'static str {
        match self {
            Self::De => "DE",
            Self::Es => "ES",
            Self::En => "EN",
            Self::Fr => "FR",
            Self::It => "IT",
            Self::Ja => "JA",
            Self::Nl => "NL",
            Self::Pt => "PT-BR",
            Self::Ru => "RU",
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct DeeplConfig {
    deepl_api_key: String,
}

impl DeeplConfig {
    fn new() -> std::io::Result<Self> {
        use std::io::Read;
        use std::path::PathBuf;
        let config_files = [
            PathBuf::new().join(".deepl"),
            dirs::home_dir().unwrap_or(PathBuf::new()).join(".deepl"),
        ];

        for config_file in config_files {
            if let Ok(mut file) = std::fs::File::open(&config_file) {
                log::debug!("Config file {:?} found.", &config_file);
                // Read .deepl as TOML
                let mut config = String::new();
                file.read_to_string(&mut config)?;
                let deepl_config: DeeplConfig = toml::from_str(&config)?;

                return Ok(deepl_config);
            } else {
                log::debug!("Config file {:?} NOT found.", &config_file);
            }
        }

        // Config file not found
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    }

    // DeepL endpoint URL
    fn endpoint(&self) -> &'static str {
        if self.deepl_api_key.ends_with(":fx") {
            // API free plan key
            "https://api-free.deepl.com/v2/translate"
        } else {
            // API Pro key
            "https://api.deepl.com/v2/translate"
        }
    }
}

/// DeepL response JSON
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct DeeplResponse {
    translations: Vec<DeeplResponseTranslation>,
}

/// DeepL response JSON for each translations
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct DeeplResponseTranslation {
    #[allow(dead_code)]
    detected_source_language: String,
    text: String,
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn plain_text_translation() {
        let deepl = Deepl::new().unwrap();

        let resp = deepl
            .translate(Language::En, Language::De, "Hello, World!")
            .await
            .unwrap();
        assert_eq!(&resp, "Hallo, Welt!");
    }
}
