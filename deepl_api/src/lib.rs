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
        let mut result = self
            .translate_strings(from_lang, to_lang, &vec![body])
            .await?;
        if 0 < result.len() {
            Ok(result.swap_remove(0))
        } else {
            // Empty response
            Ok(String::new())
        }
    }

    pub async fn translate_strings(
        &self,
        from_lang: Language,
        to_lang: Language,
        body: &Vec<&str>,
    ) -> reqwest::Result<Vec<String>> {
        let client = reqwest::Client::new();

        let mut params = vec![
            ("source_lang", from_lang.as_langcode()),
            ("target_lang", to_lang.as_langcode()),
            ("formality", "prefer_less"),
        ];

        // add texts to be translated
        for t in body {
            params.push(("text", *t));
        }

        // Make DeepL API request
        let resp = client
            .post(self.config.endpoint("translate"))
            .header(
                "authorization",
                format!("DeepL-Auth-Key {}", self.config.api_key),
            )
            .form(&params)
            .send()
            .await?;

        // Parse response
        let deepl_resp = resp.json::<DeeplTranslationResponse>().await?;
        Ok(deepl_resp
            .translations
            .into_iter()
            .map(|t| t.text)
            .collect())
    }

    /// Translate XML string
    pub async fn translate_xml(
        &self,
        from_lang: Language,
        to_lang: Language,
        xml_body: &str,
    ) -> reqwest::Result<String> {
        let client = reqwest::Client::new();

        // Make DeepL API request
        let resp = client
            .post(self.config.endpoint("translate"))
            .header(
                "authorization",
                format!("DeepL-Auth-Key {}", self.config.api_key),
            )
            .form(&[
                ("source_lang", from_lang.as_langcode()),
                ("target_lang", to_lang.as_langcode()),
                ("formality", "prefer_less"),
                ("formality", "prefer_less"),
                ("tag_handling", "xml"),
                ("ignore_tags", "header,embed,object"),
                ("text", xml_body),
            ])
            .send()
            .await?;

        // Parse response
        let mut deepl_resp = resp.json::<DeeplTranslationResponse>().await?;
        if 0 < deepl_resp.translations.len() {
            Ok(deepl_resp.translations.swap_remove(0).text)
        } else {
            // Empty response
            Ok(String::new())
        }
    }

    /// List registered glossaries
    pub async fn list_glossaries(&self) -> reqwest::Result<Vec<DeeplGlossary>> {
        let client = reqwest::Client::new();

        // Make DeepL API request
        let resp = client
            .get(self.config.endpoint("glossaries"))
            .header(
                "authorization",
                format!("DeepL-Auth-Key {}", self.config.api_key),
            )
            .send()
            .await?;

        // Parse response
        let deepl_resp = resp.json::<DeeplListGlossariesResponse>().await?;
        Ok(deepl_resp.glossaries)
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
    PtBr,
    Ru,
}

impl Language {
    fn as_langcode(&self) -> &'static str {
        match self {
            Self::De => "de",
            Self::Es => "es",
            Self::En => "en",
            Self::Fr => "fr",
            Self::It => "it",
            Self::Ja => "ja",
            Self::Nl => "nl",
            Self::Pt => "pt-br",
            Self::PtBr => "pt-br",
            Self::Ru => "ru",
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct DeeplConfig {
    api_key: String,
    glossaries: std::collections::HashMap<String,String>
}

impl DeeplConfig {
    // Search default config file
    fn new() -> std::io::Result<Self> {
        use std::path::PathBuf;
        let config_files = [
            PathBuf::new().join("deepl.toml"),
            dirs::home_dir()
                .unwrap_or(PathBuf::new())
                .join(".deepl.toml"),
        ];

        for config_file in config_files {
            match Self::new_with_config(&config_file) {
                Ok(conf) => {
                    log::debug!("Read config file {:?}", config_file);
                    return Ok(conf);
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::NotFound {
                        log::debug!("Config file {:?} NOT found.", &config_file);
                    } else {
                        // Other err, stop searching
                        log::error!("Can not parse config file {:?} : {:?}", &config_file, err);
                        return Err(err);
                    }
                }
            }
        }

        // Config file not found
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    }

    // Config from specific file
    fn new_with_config<P: AsRef<std::path::Path>>(config_path: P) -> std::io::Result<Self> {
        use std::io::Read;
        let mut file = std::fs::File::open(&config_path)?;

        // Read .deepl as TOML
        let mut config = String::new();
        file.read_to_string(&mut config)?;
        let deepl_config: DeeplConfig = toml::from_str(&config)?;

        Ok(deepl_config)
    }

    // DeepL endpoint URL
    fn endpoint(&self, api: &str) -> String {
        if self.api_key.ends_with(":fx") {
            // API free plan key
            format!("https://api-free.deepl.com/v2/{}", api)
        } else {
            // API Pro key
            format!("https://api.deepl.com/v2/{}", api)
        }
    }
}

/// DeepL translation response JSON
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct DeeplTranslationResponse {
    translations: Vec<DeeplTranslationResponseInner>,
}

/// DeepL response JSON for each translations
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct DeeplTranslationResponseInner {
    #[allow(dead_code)]
    detected_source_language: String,
    text: String,
}

/// DeepL list glossaries response JSON
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct DeeplListGlossariesResponse {
    glossaries: Vec<DeeplGlossary>,
}

/// DeepL response JSON for each glossaries
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct DeeplGlossary {
    pub glossary_id: String,
    pub name: String,
    pub ready: bool,
    pub source_lang: String,
    pub target_lang: String,
    pub creation_time: String,
    pub entry_count: i32,
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
