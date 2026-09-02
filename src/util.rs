use chromiumoxide::browser::BrowserConfigBuilder;
use chromiumoxide::{BrowserConfig, Page};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub fn get_module_data_dir() -> Result<PathBuf, Box<dyn Error>> {
    let home = dirs::home_dir().ok_or("Could not resolve home directory")?;
    Ok(home
        .join(".hearthstone-battlenet-weekly-reward")
        .join("chrome-profile"))
}

pub async fn wait_for_selector(
    page: &Page,
    selector: &str,
    timeout_millis: u128,
) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();

    while started.elapsed().as_millis() < timeout_millis {
        if page.find_element(selector).await.is_ok() {
            println!(
                "Selector found: {}, took {} ms",
                selector,
                started.elapsed().as_millis()
            );

            return Ok(());
        }

        sleep(Duration::from_millis(200)).await;
    }

    Err(format!(
        "Timeout waiting for selector: {}, took {} ms",
        selector,
        started.elapsed().as_millis()
    )
    .into())
}

pub fn create_partial_browser_config(profile_dir: &Path) -> BrowserConfigBuilder {
    BrowserConfig::builder()
        .viewport(None)
        .window_size(1920, 1080)
        .arg(format!("--user-data-dir={}", profile_dir.display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-background-timer-throttling")
        .arg("--disable-backgrounding-occluded-windows")
        .arg("--disable-renderer-backgrounding")
        .arg("--force-color-profile=srgb")
}
