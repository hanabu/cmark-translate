// SPDX-License-Identifier: MIT
//!
//! DeepL REST API wrapper
//!

pub struct Deepl {
    config: DeeplConfig,
}

impl Deepl {
    // New DeepL instance from default config file (deepl.toml or ~/.deepl.toml)
    pub fn new() -> std::io::Result<Self> {
        let deepl_config = DeeplConfig::new()?;

        Ok(Self {
            config: deepl_config,
        })
    }

    /// New DeepL instance from specific config file
    pub fn with_config<P: AsRef<std::path::Path>>(config_path: P) -> std::io::Result<Self> {
        let deepl_config = DeeplConfig::with_config(config_path)?;

        Ok(Self {
            config: deepl_config,
        })
    }

    /// Translate single text string
    #[allow(dead_code)]
    pub async fn translate(
        &self,
        from_lang: Language,
        to_lang: Language,
        formality: Formality,
        body: &str,
    ) -> reqwest::Result<String> {
        let mut result = self
            .translate_strings(from_lang, to_lang, formality, &vec![body])
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
        formality: Formality,
        body: &Vec<&str>,
    ) -> reqwest::Result<Vec<String>> {
        let mut params = vec![
            ("source_lang", from_lang.as_src_langcode()),
            ("target_lang", to_lang.as_langcode()),
            ("preserve_formatting", "1"),
            ("formality", formality.to_str()),
        ];
        if let Some(glossary_id) = self.config.glossary(from_lang, to_lang) {
            log::debug!("Use glossary {}", glossary_id);
            params.push(("glossary_id", glossary_id));
        }

        // add texts to be translated
        for t in body {
            params.push(("text", *t));
        }

        // Make DeepL API request
        let client = reqwest::Client::new();
        let resp = client
            .post(self.config.endpoint("translate"))
            .header(
                "authorization",
                format!("DeepL-Auth-Key {}", self.config.api_key),
            )
            .form(&params)
            .send()
            .await?;

        // Returns error
        resp.error_for_status_ref()?;

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
        formality: Formality,
        xml_body: &str,
    ) -> reqwest::Result<String> {
        // Prepare request parameters
        let mut params = vec![
            ("source_lang", from_lang.as_src_langcode()),
            ("target_lang", to_lang.as_langcode()),
            ("preserve_formatting", "1"),
            ("formality", formality.to_str()),
            ("tag_handling", "xml"),
            ("ignore_tags", "header,embed,object"),
            (
                "splitting_tags",
                "blockquote,li,dt,dd,p,h1,h2,h3,h4,h5,h6,th,td",
            ),
            ("non_splitting_tags", "embed,em,strong,del,a,img"),
        ];
        if let Some(glossary_id) = self.config.glossary(from_lang, to_lang) {
            log::debug!("Use glossary {}", glossary_id);
            params.push(("glossary_id", glossary_id));
        }
        params.push(("text", xml_body));

        // Make DeepL API request
        let client = reqwest::Client::new();
        let resp = client
            .post(self.config.endpoint("translate"))
            .header(
                "authorization",
                format!("DeepL-Auth-Key {}", self.config.api_key),
            )
            .form(&params)
            .send()
            .await?;

        // Returns error
        resp.error_for_status_ref()?;

        // Parse response
        let mut deepl_resp = resp.json::<DeeplTranslationResponse>().await?;
        if 0 < deepl_resp.translations.len() {
            Ok(deepl_resp.translations.swap_remove(0).text)
        } else {
            // Empty response
            Ok(String::new())
        }
    }

    /// Register new glossary
    pub async fn register_glossaries<S: AsRef<str>>(
        &self,
        name: &str,
        from_lang: Language,
        to_lang: Language,
        glossaries: &[(S, S)],
    ) -> reqwest::Result<DeeplGlossary> {
        // Remove spaces, empty items
        let mut filtered_glossaries = glossaries
            .iter()
            .filter_map(|(from, to)| {
                let from_trimed = from.as_ref().trim();
                let to_trimed = to.as_ref().trim();
                if from_trimed.is_empty() || to_trimed.is_empty() {
                    None
                } else {
                    Some((from_trimed, to_trimed))
                }
            })
            .collect::<Vec<_>>();

        // Check duplicates
        filtered_glossaries.sort_by(|(from1, _), (from2, _)| from1.cmp(from2));
        filtered_glossaries.iter().fold("", |prev_from, (from, _)| {
            if prev_from == *from {
                // Duplicated
                log::warn!("Duplicated key : \"{}\"", *from);
            }
            *from
        });

        // Make TSV text
        let tsv: String = filtered_glossaries
            .iter()
            .map(|(from, to)| {
                let row = format!("{}\t{}", from, to);
                log::trace!("TSV: {}", row);
                row
            })
            .collect::<Vec<String>>()
            .join("\n");

        // Make DeepL API request
        let client = reqwest::Client::new();
        let resp = client
            .post(self.config.endpoint("glossaries"))
            .header(
                "authorization",
                format!("DeepL-Auth-Key {}", self.config.api_key),
            )
            .form(&[
                ("name", name),
                ("source_lang", from_lang.as_src_langcode()),
                ("target_lang", to_lang.as_langcode()),
                ("entries_format", "tsv"),
                ("entries", &tsv),
            ])
            .send()
            .await?;

        if let Err(err) = resp.error_for_status_ref() {
            // Returns error with printing details
            if let Ok(err_body_text) = resp.text().await {
                log::error!("{}", err_body_text);
            }
            Err(err)
        } else {
            // Success, parse response
            let deepl_resp = resp.json::<DeeplGlossary>().await?;
            Ok(deepl_resp)
        }
    }

    /// List registered glossaries
    pub async fn list_glossaries(&self) -> reqwest::Result<Vec<DeeplGlossary>> {
        // Make DeepL API request
        let client = reqwest::Client::new();
        let resp = client
            .get(self.config.endpoint("glossaries"))
            .header(
                "authorization",
                format!("DeepL-Auth-Key {}", self.config.api_key),
            )
            .send()
            .await?;

        // Returns error
        resp.error_for_status_ref()?;

        // Parse response
        let deepl_resp = resp.json::<DeeplListGlossariesResponse>().await?;
        Ok(deepl_resp.glossaries)
    }

    /// Remove registered glossaries
    pub async fn remove_glossary(&self, id: &str) -> reqwest::Result<()> {
        // Make DeepL API request
        let client = reqwest::Client::new();
        let resp = client
            .delete(self.config.endpoint(&format!("glossaries/{}", id)))
            .header(
                "authorization",
                format!("DeepL-Auth-Key {}", self.config.api_key),
            )
            .send()
            .await?;

        // Check response
        resp.error_for_status()?;

        Ok(())
    }

    /// Get usage, returns translated characters
    pub async fn get_usage(&self) -> reqwest::Result<i32> {
        // Make DeepL API request
        let client = reqwest::Client::new();
        let resp = client
            .get(self.config.endpoint("usage"))
            .header(
                "authorization",
                format!("DeepL-Auth-Key {}", self.config.api_key),
            )
            .send()
            .await?;

        // Returns error
        resp.error_for_status_ref()?;

        // Parse response
        let deepl_resp = resp.json::<DeeplUsageResponse>().await?;
        Ok(deepl_resp.character_count)
    }
}

#[derive(Clone, Copy, serde::Deserialize)]
pub enum Language {
    Ar,     // Arabic
    Bg,     // Bulgarian
    Cs,     // Czech
    Da,     // Danish
    De,     // German
    El,     // Greek
    En,     // English (unspecified variant)
    EnGb,   // English (British)
    EnUs,   // English (American)
    Es,     // Spanish
    Et,     // Estonian
    Fi,     // Finnish
    Fr,     // French
    Hu,     // Hungarian
    Id,     // Indonesian
    It,     // Italian
    Ja,     // Japanese
    Ko,     // Korean
    Lt,     // Lithuanian
    Lv,     // Latvian
    Nb,     // Norwegian Bokmål
    Nl,     // Dutch
    Pl,     // Polish
    Pt,     // Portuguese (unspecified variant)
    PtBr,   // Portuguese (Brazilian)
    PtPt,   // Portuguese (Pt excluding Brazilian Portuguese)
    Ro,     // Romanian
    Ru,     // Russian
    Sk,     // Slovak
    Sl,     // Slovenian
    Sv,     // Swedish
    Tr,     // Turkish
    Uk,     // Ukrainian
    Zh,     // Chinese (unspecified variant)
    ZhHans, // Chinese (simplified)
    ZhHant, // Chinese (traditional)
}

impl Language {
    /// DeepL supported target language code
    pub fn as_langcode(&self) -> &'static str {
        match self {
            Self::Ar => "ar",
            Self::Bg => "bg",
            Self::Cs => "cs",
            Self::Da => "da",
            Self::De => "de",
            Self::El => "el",
            Self::En => "en-us",
            Self::EnGb => "en-gb",
            Self::EnUs => "en-us",
            Self::Es => "es",
            Self::Et => "et",
            Self::Fi => "fi",
            Self::Fr => "fr",
            Self::Hu => "hu",
            Self::Id => "id",
            Self::It => "it",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Lt => "lt",
            Self::Lv => "lv",
            Self::Nb => "nb",
            Self::Nl => "nl",
            Self::Pl => "pl",
            Self::Pt => "pt-br",
            Self::PtBr => "pt-br",
            Self::PtPt => "pt-pt",
            Self::Ro => "ro",
            Self::Ru => "ru",
            Self::Sk => "sk",
            Self::Sl => "sl",
            Self::Sv => "sv",
            Self::Tr => "tr",
            Self::Uk => "uk",
            Self::Zh => "zh-hans",
            Self::ZhHans => "zh-hans",
            Self::ZhHant => "zh-hant",
        }
    }

    /// DeepL supported source language code
    pub fn as_src_langcode(&self) -> &'static str {
        match self {
            Self::En | Self::EnGb | Self::EnUs => "en",
            Self::Pt | Self::PtBr | Self::PtPt => "pt",
            Self::Zh | Self::ZhHans | Self::ZhHant => "zh",
            _ => self.as_langcode(),
        }
    }
}

impl std::str::FromStr for Language {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lowcase = s.to_ascii_lowercase();
        match lowcase.as_str() {
            "ar" => Ok(Self::Ar),
            "bg" => Ok(Self::Bg),
            "cs" => Ok(Self::Cs),
            "da" => Ok(Self::Da),
            "de" => Ok(Self::De),
            "el" => Ok(Self::El),
            "en" => Ok(Self::En),
            "en-gb" => Ok(Self::EnGb),
            "en-us" => Ok(Self::EnUs),
            "es" => Ok(Self::Es),
            "et" => Ok(Self::Et),
            "fi" => Ok(Self::Fi),
            "fr" => Ok(Self::Fr),
            "hu" => Ok(Self::Hu),
            "id" => Ok(Self::Id),
            "it" => Ok(Self::It),
            "ja" => Ok(Self::Ja),
            "ko" => Ok(Self::Ko),
            "lt" => Ok(Self::Lt),
            "lv" => Ok(Self::Lv),
            "nb" => Ok(Self::Nb),
            "nl" => Ok(Self::Nl),
            "pl" => Ok(Self::Pl),
            "pt" => Ok(Self::Pt),
            "pt-br" => Ok(Self::PtBr),
            "pt-pt" => Ok(Self::PtPt),
            "ro" => Ok(Self::Ro),
            "ru" => Ok(Self::Ru),
            "sk" => Ok(Self::Sk),
            "sl" => Ok(Self::Sl),
            "sv" => Ok(Self::Sv),
            "tr" => Ok(Self::Tr),
            "uk" => Ok(Self::Uk),
            "zh" => Ok(Self::Zh),
            "zh-hans" => Ok(Self::ZhHans),
            "zh-hant" => Ok(Self::ZhHant),
            _ => Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
        }
    }
}

/// Translation output formality
#[derive(Clone, Copy, serde::Deserialize)]
pub enum Formality {
    Default,
    Formal,
    Informal,
}

impl Formality {
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Formal => "prefer_more",
            Self::Informal => "prefer_less",
        }
    }
}

impl Default for Formality {
    fn default() -> Self {
        Self::Default
    }
}

impl std::str::FromStr for Formality {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lowcase = s.to_ascii_lowercase();
        match lowcase.as_str() {
            "default" => Ok(Self::Default),
            "formal" => Ok(Self::Formal),
            "informal" => Ok(Self::Informal),
            _ => Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct DeeplConfig {
    api_key: String,
    glossaries: std::collections::HashMap<String, String>,
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
            match Self::with_config(&config_file) {
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
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "deepl.toml NOT found",
        ))
    }

    // Config from specific file
    fn with_config<P: AsRef<std::path::Path>>(config_path: P) -> std::io::Result<Self> {
        use std::io::Read;
        let mut file = std::fs::File::open(&config_path)?;

        // Read .deepl as TOML
        let mut config = String::new();
        file.read_to_string(&mut config)?;
        let deepl_config: DeeplConfig = toml::from_str(&config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

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

    // Find glossary
    fn glossary<'a>(&'a self, from_lang: Language, to_lang: Language) -> Option<&'a str> {
        let glossary_key = format!("{}_{}", from_lang.as_src_langcode(), to_lang.as_langcode());
        self.glossaries.get(&glossary_key).map(|v| v.as_str())
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

/// DeepL usage response JSON
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct DeeplUsageResponse {
    character_count: i32,
    #[allow(dead_code)]
    character_limit: i32,
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn plain_text_translation() {
        let deepl = Deepl::new().unwrap();

        let resp = deepl
            .translate(
                Language::En,
                Language::De,
                Formality::Default,
                "Hello, World!",
            )
            .await
            .unwrap();
        assert_eq!(&resp, "Hallo, Welt!");
    }
}
