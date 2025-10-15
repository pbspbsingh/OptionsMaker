use std::{sync::Arc, thread, time::Duration};

use askama::Template;
use headless_chrome::{Browser, Tab, browser::tab::ModifierKey};
use serde_json::Value;

const GEMINI_URL: &str = "https://gemini.google.com/app";

pub fn load_gemini(browser: Arc<Browser>) -> anyhow::Result<Arc<Tab>> {
    let tab = browser.new_tab()?;
    tab.navigate_to(GEMINI_URL)?
        .wait_until_navigated()?
        .press_key("Escape")?;
    Ok(tab)
}

#[derive(Template)]
#[template(path = "fundamental_query.md")]
struct FundamentalQuery {
    symbol: String,
}

pub fn ask_ai(tab: Arc<Tab>, symbol: String) -> anyhow::Result<String> {
    delay();

    tab.navigate_to(GEMINI_URL)?
        .wait_until_navigated()?
        .press_key("Escape")?;
    delay();

    tab.press_key_with_modifiers("o", Some(&[ModifierKey::Ctrl, ModifierKey::Shift]))?;
    delay();

    let input_field = tab.wait_for_element(r#"rich-textarea.text-input-field_textarea"#)?;
    input_field.click()?;
    delay();

    let query = FundamentalQuery {
        symbol: symbol.clone(),
    };
    tab.send_character(&query.render()?)?;
    delay();

    tab.press_key_with_modifiers("Enter", Some(&[ModifierKey::Ctrl]))?;
    delay();

    tab.wait_for_element_with_custom_timeout(
        r#"mat-icon[fonticon="thumb_up"]"#,
        Duration::from_secs(300),
    )?;
    delay();

    let response = tab.evaluate(r#"
        let responses = document.querySelectorAll('response-container div.response-container-content');
        responses[responses.length - 1].innerHTML
    "#, false)?;
    if let Some(Value::String(html)) = response.value {
        return Ok(html);
    }
    anyhow::bail!("Fail to fetch the response for {symbol}")
}

fn delay() {
    thread::sleep(Duration::from_millis(rand::random_range(400..2000)));
}
