use chromiumoxide::browser::BrowserConfigBuilder;
use chromiumoxide::{BrowserConfig, Page};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, warn};

pub fn get_module_data_dir() -> Result<PathBuf, Box<dyn Error>> {
    let home = dirs::home_dir().ok_or("Could not resolve home directory")?;
    Ok(home.join(".hearthstone-battlenet-weekly-reward").join("chrome-profile"))
}

pub async fn wait_for_selector(page: &Page, selector: &str, timeout_millis: u128) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    let mut attempt = 0;

    while started.elapsed().as_millis() < timeout_millis {
        if page.find_element(selector).await.is_ok() {
            debug!(selector, elapsed_ms = started.elapsed().as_millis(), attempt, "Selector found.");
            return Ok(());
        }

        attempt += 1;
        debug!(selector, attempt, "Waiting for selector to appear...");
        sleep(Duration::from_millis(1000)).await;
    }

    warn!(selector, elapsed_ms = started.elapsed().as_millis(), "Selector timeout reached.");
    Err(format!("Timeout waiting for selector: {}, took {} ms", selector, started.elapsed().as_millis()).into())
}

pub fn create_partial_browser_config(profile_dir: &Path) -> BrowserConfigBuilder {
    BrowserConfig::builder()
        .viewport(None)
        .window_size(1920, 1080)
        .user_data_dir(profile_dir)
        .arg("--no-first-run")
        .arg("--disk-cache-size=1")
        .arg("--no-default-browser-check")
        .arg("--disable-background-timer-throttling")
        .arg("--disable-backgrounding-occluded-windows")
        .arg("--disable-renderer-backgrounding")
        .arg("--force-color-profile=srgb")
        .arg("--profile-directory=Default")
        .arg("--disable-features=IsolateOrigins,site-per-process")
        .arg("--restore-last-session")
}
