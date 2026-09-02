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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let started = Instant::now();

    println!("Initializing profile directory...");
    let profile_dir = get_module_data_dir()?;
    fs::create_dir_all(&profile_dir)?;

    //configure headless browser
    let config = create_partial_browser_config(&profile_dir).build()?;

    println!("Initialized. Launching browser...");
    let (mut browser, mut handler) = Browser::launch(config).await?;

    //keep connection to browser open
    tokio::spawn(async move { while let Some(_event) = handler.next().await {} });

    println!("Launched. Opening Battle.net login page...");
    let page = browser
        .new_page("https://us.shop.battle.net/login?ref=https%3A%2F%2Fus.shop.battle.net%2Fen-us%2Ffamily%2Fhearthstone%23optLogin%3Dtrue")
        .await?;

    println!("Opened. Checking authentication state...");
    let authed = check_auth(&page).await?;
    if authed {
        println!("--> Authenticated! Executing HEADLESS reward claim...");
        run_reward_claim(page).await?;

        println!("Closing browser...");
        browser.close().await?;
    } else {
        println!("--> Unauthenticated. Executing HEADFUL manual login flow...");
        println!("Closing headless browser to open headful browser...");
        browser.close().await?;
        run_unauthenticated_flow(&profile_dir).await?;
    }

    println!("Task completed. Total time: {} ms", started.elapsed().as_millis());

    Ok(())
}

async fn check_auth(page: &Page) -> Result<bool, Box<dyn Error>> {
    //wait for what we need to mount
    if wait_for_selector(page, "blz-nav-battlenet", 7000).await.is_err() {
        return Ok(false);
    }
    wait_for_selector(page, "blz-nav-battlenet[authenticated]", 7000).await.ok();

    //verify if user is logged in by checking for absence of "Account" placeholder text
    let authed: bool = page.evaluate(r#"!!document.querySelector("blz-nav-battlenet[authenticated]")"#).await?.into_value()?;

    Ok(authed)
}

async fn run_unauthenticated_flow(profile_dir: &Path) -> Result<(), Box<dyn Error>> {
    //configure browser
    let config = create_partial_browser_config(profile_dir).with_head().build()?;

    //launch browser
    println!("Launching browser...");
    let (mut browser, mut handler) = Browser::launch(config).await?;

    //keep connection to browser open
    tokio::spawn(async move { while let Some(_event) = handler.next().await {} });

    //open login page that will then redirect to hearthstone store page after successful login
    println!("Launched. Opening Battle.net login page...");
    let page = browser
        .new_page("https://us.shop.battle.net/login?ref=https%3A%2F%2Fus.shop.battle.net%2Fen-us%2Ffamily%2Fhearthstone%23optLogin%3Dtrue")
        .await?;

    println!("--------------------------------");
    println!("Please log into your Battle.net account in the opened Chrome window...");
    println!("Waiting for authentication to complete (timeout: 5 minutes)...");
    println!("--------------------------------");

    //wait for login
    let started = Instant::now();
    while started.elapsed().as_secs() < 300 {
        //check if we are redirected back
        if page
            .url()
            .await?
            .ok_or("Could not find page url during ")?
            .starts_with("https://us.shop.battle.net/en-us/family/hearthstone")
        {
            break;
        }

        sleep(Duration::from_secs(1)).await;
    }

    let authed = check_auth(&page).await?;

    //if logged in, run reward claim
    if authed {
        println!("Authenticated. Executing HEADLESS reward claim...");
        run_reward_claim(page).await?;
        browser.close().await?;
        return Ok(());
    }

    //timeout and close
    browser.close().await?;
    Err("Login timed out after 5 minutes.".into())
}

async fn run_reward_claim(page: Page) -> Result<(), Box<dyn Error>> {
    println!("Waiting for reward claim button to be available...");
    let claim_selector = "a[aria-label=\"Shop, Hearthstone®: Battle.net® Shop: Weekly Reward\"] a[aria-label=\"Claim Free\"]";
    wait_for_selector(&page, claim_selector, 7000).await?;

    //click the claim button
    page.find_element(claim_selector).await?.click().await?;

    //check to see if the button is now disabled, indicating the reward has been claimed
    match wait_for_selector(&page, "a[aria-label=\"Shop, Hearthstone®: Battle.net® Shop: Weekly Reward\"] button[disabled]", 7000).await {
        Ok(_) => println!("Reward claimed successfully."),
        Err(e) => {
            println!("Failed to confirm reward claim: {}", e);
            return Err("Failed to confirm reward claim.".into());
        }
    }

    Ok(())
}
