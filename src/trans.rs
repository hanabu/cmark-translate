// SPDX-License-Identifier: MIT
use crate::{cmark_xml, deepl};

/// Translate CommonMark .md file
pub async fn translate_cmark_file<P: AsRef<std::path::Path>>(
    deepl: &deepl::Deepl,
    from_lang: deepl::Language,
    to_lang: deepl::Language,
    formality: deepl::Formality,
    src_path: P,
    dst_path: P,
) -> std::io::Result<()> {
    use std::io::Write;

    // Read .md file
    let mut f = std::fs::File::open(src_path)?;
    let (cmark_text, frontmatter) = cmark_xml::read_cmark_with_frontmatter(&mut f)?;
    drop(f);

    log::trace!(
        "Read file:\n+++\n{}\n+++\n{}",
        frontmatter.as_deref().unwrap_or_default(),
        cmark_text
    );

    // Parse frontmatter
    let translated_frontmatter = if let Some(frontmatter) = frontmatter {
        // translate TOML frontmatter
        Some(translate_toml(&deepl, from_lang, to_lang, formality, &frontmatter).await?)
    } else {
        None
    };

    // Translate CommonMark body
    let translated_cmark =
        translate_cmark(&deepl, from_lang, to_lang, formality, &cmark_text).await?;

    // Print result
    let mut f = std::fs::File::create(dst_path)?;
    if let Some(translated_frontmatter) = translated_frontmatter {
        f.write_all("+++\n".as_bytes())?;
        f.write_all(translated_frontmatter.as_bytes())?;
        f.write_all("+++\n".as_bytes())?;
    }
    f.write_all(translated_cmark.as_bytes())?;
    Ok(())
}

/// Translate TOML frontmatter
pub async fn translate_toml(
    deepl: &deepl::Deepl,
    from_lang: deepl::Language,
    to_lang: deepl::Language,
    formality: deepl::Formality,
    toml_frontmatter: &str,
) -> Result<String, std::io::Error> {
    if let toml::Value::Table(mut root) = toml_frontmatter.parse::<toml::Value>()? {
        // Pickup TOML key for translation
        let mut should_be_translate: Vec<&mut String> = vec![];
        for (key, val) in &mut root {
            match key.as_str() {
                "title" | "description" => {
                    if let toml::Value::String(val) = val {
                        should_be_translate.push(val);
                    }
                }
                "extra" => {
                    if let toml::Value::Table(extra) = val {
                        for (extra_key, extra_val) in extra {
                            match extra_key.as_str() {
                                "time" => {
                                    if let toml::Value::String(extra_val) = extra_val {
                                        should_be_translate.push(extra_val);
                                    }
                                }
                                _ => (),
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Prepare input Vec
        let src_vec = should_be_translate
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();
        // Translate texts
        let translated_vec = deepl
            .translate_strings(from_lang, to_lang, formality, &src_vec)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Replace TOML value with translated text
        should_be_translate
            .into_iter()
            .zip(translated_vec.iter())
            .for_each(|(toml_val, translated_str)| {
                toml_val.clear();
                *toml_val += translated_str.as_str();
            });

        // Serialize toml::Value should not fail
        let translated_frontmatter = toml::to_string_pretty(&toml::Value::Table(root)).unwrap();
        // Show translated frontmatter
        log::trace!("Translated TOML :\n{}\n", translated_frontmatter);

        Ok(translated_frontmatter)
    } else {
        // TOML parse failed
        Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
    }
}

/// Translate CommonMark
pub async fn translate_cmark(
    deepl: &deepl::Deepl,
    from_lang: deepl::Language,
    to_lang: deepl::Language,
    formality: deepl::Formality,
    cmark_text: &str,
) -> Result<String, std::io::Error> {
    let xml = cmark_xml::xml_from_cmark(&cmark_text, true);
    log::trace!("XML: {}\n", xml);

    // translate
    let xml_translated = deepl
        .translate_xml(from_lang, to_lang, formality, &xml)
        .await
        .unwrap();

    // write back to markdown format
    let cmark_translated = cmark_xml::cmark_from_xml(&xml_translated, true).unwrap();

    Ok(cmark_translated)
}
