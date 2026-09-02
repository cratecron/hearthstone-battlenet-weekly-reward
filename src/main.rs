mod util;

use crate::util::{create_partial_browser_config, get_module_data_dir, wait_for_selector};
use chromiumoxide::Page;
use chromiumoxide::browser::Browser;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //init logging
    fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,chromiumoxide=error")))
        .with_target(false)
        .try_init()
        .ok();

    info!("Initializing profile directory...");
    let profile_dir = get_module_data_dir()?;
    fs::create_dir_all(&profile_dir)?;

    //configure headless browser
    let config = create_partial_browser_config(&profile_dir).build()?;

    info!("Initialized. Launching browser...");
    let (mut browser, mut handler) = Browser::launch(config).await?;

    //keep connection to browser open
    tokio::spawn(async move { while let Some(_event) = handler.next().await {} });

    info!("Launched. Opening Battle.net login page...");
    let page = browser.new_page("https://us.shop.battle.net/login?ref=https%3A%2F%2Fus.shop.battle.net%2Fen-us%2Ffamily%2Fhearthstone%23optLogin%3Dtrue").await?;

    info!("Opened. Checking authentication state...");
    let authed = check_auth(&page).await?;
    if authed {
        info!("--> Authenticated! Executing HEADLESS reward claim...");
        run_reward_claim(page).await?;

        info!("Closing browser...");
        browser.close().await?;
    } else {
        info!("--> Unauthenticated. Executing HEADFUL manual login flow...");
        info!("Closing headless browser to open headful browser...");
        browser.close().await?;
        run_unauthenticated_flow(&profile_dir).await?;
    }

    Ok(())
}

async fn check_auth(page: &Page) -> Result<bool, Box<dyn Error>> {
    //wait for what we need to mount
    if wait_for_selector(page, "blz-nav-battlenet", 7000).await.is_err() {
        warn!("Battle.net nav did not mount in time; assuming unauthenticated.");
        return Ok(false);
    }

    if wait_for_selector(page, "blz-nav-battlenet[authenticated]", 7000).await.is_err() {
        warn!("Battle.net nav did not indicate authenticated state in time; assuming unauthenticated.");
        return Ok(false);
    }

    //verify if user is logged in by checking for absence of "Account" placeholder text
    let authed: bool = page.evaluate(r#"!!document.querySelector("blz-nav-battlenet[authenticated]")"#).await?.into_value()?;
    debug!(authed, "Authentication state determined.");

    Ok(authed)
}

async fn run_unauthenticated_flow(profile_dir: &Path) -> Result<(), Box<dyn Error>> {
    //configure browser
    let config = create_partial_browser_config(profile_dir).with_head().build()?;

    //launch browser
    info!("Launching browser...");
    let (mut browser, mut handler) = Browser::launch(config).await?;

    //keep connection to browser open
    tokio::spawn(async move { while let Some(_event) = handler.next().await {} });

    //open login page that will then redirect to hearthstone store page after successful login
    info!("Launched. Opening Battle.net login page...");
    let page = browser.new_page("https://us.shop.battle.net/login?ref=https%3A%2F%2Fus.shop.battle.net%2Fen-us%2Ffamily%2Fhearthstone%23optLogin%3Dtrue").await?;

    info!("--------------------------------");
    info!("Please log into your Battle.net account in the opened Chrome window...");
    info!("Waiting for authentication to complete (timeout: 5 minutes)...");
    info!("--------------------------------");

    //wait for login
    let started = Instant::now();
    while started.elapsed().as_secs() < 300 {
        //check if we are redirected back
        if page.url().await?.ok_or("Could not find page url")?.starts_with("https://us.shop.battle.net/en-us/family/hearthstone") {
            debug!("Redirected back to the Hearthstone store page.");
            break;
        }

        sleep(Duration::from_secs(1)).await;
    }

    //check if we are authenticated
    info!("Checking authentication state...");
    let authed = check_auth(&page).await?;

    //if logged in, run reward claim
    if authed {
        info!("--> Authenticated. Executing HEADLESS reward claim...");
        run_reward_claim(page).await?;
        browser.close().await?;
        return Ok(());
    }

    //timeout and close
    warn!("Login timed out after 5 minutes.");
    browser.close().await?;
    Err("Login timed out after 5 minutes.".into())
}

async fn run_reward_claim(page: Page) -> Result<(), Box<dyn Error>> {
    info!("Waiting for reward claim button to be available...");
    let claim_selector = "a[aria-label=\"Shop, Hearthstone®: Battle.net® Shop: Weekly Reward\"] a[aria-label=\"Claim Free\"]";
    wait_for_selector(&page, claim_selector, 7000).await?;

    //click the claim button
    info!(selector = %claim_selector, "Clicking reward claim button...");
    page.find_element(claim_selector).await?.click().await?;

    //check to see if the button is now disabled, indicating the reward has been claimed
    match wait_for_selector(&page, "a[aria-label=\"Shop, Hearthstone®: Battle.net® Shop: Weekly Reward\"] button[disabled]", 7000).await {
        Ok(_) => info!("Reward claimed successfully."),
        Err(e) => {
            error!("Failed to confirm reward claim: {}", e);
            return Err("Failed to confirm reward claim.".into());
        }
    }

    Ok(())
}
