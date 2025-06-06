use std::path::Path;

use crate::{error::Error, formats::*, traits::Parser, types::Resource};

pub struct Codec {
    pub resources: Box<Vec<Resource>>,
}

impl Codec {
    pub fn new() -> Self {
        Codec {
            resources: Box::new(Vec::new()),
        }
    }
}

// MARK: Data I/O

impl Codec {
    pub fn read_file_by_type<P: AsRef<Path>>(
        &mut self,
        path: P,
        format_type: FormatType,
    ) -> Result<(), Error> {
        // If lang is None, we will try to infer the language from the path.
        // The finding logic is as follows:
        // Split the path by the component separator (usually '/'),
        // and look for the language identifier from last to first.
        // For Apple platforms, we will look pattern like "en.lproj" or "fr.lproj".
        // For Android, we will look for "values-en" or "values-fr".
        let language = match &format_type {
            FormatType::AndroidStrings(lang) | FormatType::Strings(lang) => {
                let processed_lang = if let Some(lang) = lang {
                    lang.clone()
                } else {
                    path.as_ref()
                        .components()
                        .rev()
                        .find_map(|c| {
                            let component = c.as_os_str().to_str()?;
                            if component.ends_with(".lproj") {
                                Some(component.trim_end_matches(".lproj").to_string())
                            } else if component.starts_with("values-") {
                                Some(component.trim_start_matches("values-").to_string())
                            } else {
                                None
                            }
                        })
                        .ok_or(Error::UnknownFormat(
                            "Failed to infer language from path, please provide a language code manually."
                                .to_string(),
                        ))?
                };

                Some(processed_lang)
            }
            _ => None,
        };

        let domain = path
            .as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let path = path.as_ref();

        let mut new_resources = match &format_type {
            FormatType::Strings(_) => {
                vec![Resource::from(StringsFormat::read_from(path)?)]
            }
            FormatType::AndroidStrings(_) => {
                vec![Resource::from(AndroidStringsFormat::read_from(path)?)]
            }
            FormatType::Xcstrings => Vec::<Resource>::try_from(XcstringsFormat::read_from(path)?)?,
        };

        for new_resource in &mut new_resources {
            if let Some(ref lang) = language {
                new_resource.metadata.language = lang.clone();
            }
            new_resource.metadata.domain = domain.clone();
            new_resource
                .metadata
                .custom
                .insert("format".to_string(), format_type.to_string());
        }
        self.resources.append(&mut new_resources);

        Ok(())
    }

    /// Read a file by its path and infer the format based on the file extension.
    /// If the language is not provided, it will try to infer it from the path.
    pub fn read_file_by_extension<P: AsRef<Path>>(
        &mut self,
        path: P,
        lang: Option<String>,
    ) -> Result<(), Error> {
        let format_type = match path.as_ref().extension().and_then(|s| s.to_str()) {
            Some("xml") => FormatType::AndroidStrings(lang),
            Some("strings") => FormatType::Strings(lang),
            Some("xcstrings") => FormatType::Xcstrings,
            extension => {
                return Err(Error::UnsupportedFormat(format!(
                    "Unsupported file extension: {:?}.",
                    extension
                )));
            }
        };

        self.read_file_by_type(path, format_type)?;

        Ok(())
    }
}

// For format which only has one resource per file, we would consider the vector only has one element.
// and for format which has multiple resources per file, we would consider the vector has multiple elements.
fn write_resources_to_file(resources: &[Resource], file_path: &String) -> Result<(), Error> {
    let path = Path::new(&file_path);

    if let Some(first) = resources.first() {
        match first.metadata.custom.get("format").map(String::as_str) {
            Some("AndroidStrings") => AndroidStringsFormat::from(first.clone()).write_to(path)?,
            Some("Strings") => StringsFormat::try_from(first.clone())?.write_to(path)?,
            Some("Xcstrings") => XcstringsFormat::try_from(resources.to_vec())?.write_to(path)?,
            _ => Err(Error::UnsupportedFormat(format!(
                "Unsupported format: {:?}",
                first.metadata.custom.get("format")
            )))?,
        }
    }

    Ok(())
}

impl Codec {
    /// Write the resources back to the original file
    pub fn write_to_file(&self) -> Result<(), Error> {
        // Group resources by the domain in a HashMap
        let mut grouped_resources: std::collections::HashMap<String, Vec<Resource>> =
            std::collections::HashMap::new();
        for resource in &*self.resources {
            let domain = resource.metadata.domain.clone();
            grouped_resources
                .entry(domain)
                .or_insert_with(Vec::new)
                .push(resource.clone());
        }

        // Iterate the map and write each resource to its respective file
        for (domain, resources) in grouped_resources {
            write_resources_to_file(&resources, &domain)?;
        }

        Ok(())
    }
}
// MARK: Data Caching and Loading

impl Codec {
    /// Cache the current resources to a file in JSON format.
    pub fn cache_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let mut writer = std::fs::File::create(path).map_err(Error::Io)?;
        serde_json::to_writer(&mut writer, &*self.resources).map_err(Error::Parse)?;
        Ok(())
    }

    /// Load resources from a JSON file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let mut reader = std::fs::File::open(path).map_err(Error::Io)?;
        let resources: Box<Vec<Resource>> =
            serde_json::from_reader(&mut reader).map_err(Error::Parse)?;
        Ok(Codec { resources })
    }
}
