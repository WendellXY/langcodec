use crate::{Codec, Error};

#[derive(Debug, Clone)]
pub struct NormalizeOptions {
    pub normalize_placeholders: bool,
    pub key_style: KeyStyle,
}

impl Default for NormalizeOptions {
    fn default() -> Self {
        Self {
            normalize_placeholders: true,
            key_style: KeyStyle::None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum KeyStyle {
    #[default]
    None,
    Snake,
    Kebab,
    Camel,
}

#[derive(Debug, Clone, Default)]
pub struct NormalizeReport {
    pub changed: bool,
}

pub fn normalize_codec(
    codec: &mut Codec,
    _options: &NormalizeOptions,
) -> Result<NormalizeReport, Error> {
    let mut changed = false;

    for resource in &mut codec.resources {
        let before_order: Vec<String> = resource.entries.iter().map(|entry| entry.id.clone()).collect();
        resource.entries.sort_by(|left, right| left.id.cmp(&right.id));
        let after_order: Vec<String> = resource.entries.iter().map(|entry| entry.id.clone()).collect();
        if before_order != after_order {
            changed = true;
        }
    }

    Ok(NormalizeReport { changed })
}
