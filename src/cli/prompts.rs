//! Low-level prompt helpers wrapping `dialoguer`.

use anyhow::Result;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

pub fn ask_string(prompt: &str, default: Option<&str>) -> Result<String> {
    let theme = ColorfulTheme::default();
    let mut builder = Input::<String>::with_theme(&theme).with_prompt(prompt);
    if let Some(d) = default {
        builder = builder.default(d.to_string());
    }
    Ok(builder.interact_text()?)
}

pub fn ask_optional(prompt: &str, hint: &str) -> Result<Option<String>> {
    let theme = ColorfulTheme::default();
    let val: String = Input::<String>::with_theme(&theme)
        .with_prompt(format!("{} ({})", prompt, hint))
        .allow_empty(true)
        .interact_text()?;
    Ok(if val.trim().is_empty() { None } else { Some(val.trim().to_string()) })
}

pub fn ask_select<T: ToString>(prompt: &str, items: &[T]) -> Result<usize> {
    let theme = ColorfulTheme::default();
    Ok(Select::with_theme(&theme)
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()?)
}

pub fn ask_confirm(prompt: &str, default: bool) -> Result<bool> {
    let theme = ColorfulTheme::default();
    Ok(Confirm::with_theme(&theme)
        .with_prompt(prompt)
        .default(default)
        .interact()?)
}
