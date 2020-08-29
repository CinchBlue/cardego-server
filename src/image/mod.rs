extern crate cairo;
extern crate pango;
extern crate pangocairo;
extern crate reqwest;
extern crate anyhow;
extern crate log;
extern crate regex;

pub mod templates;

use crate::models::Card;
use crate::image::templates::{SingleCardTemplate, CardsheetTemplate};

use askama::Template;

use cairo::{ImageSurface, FontSlant, FontWeight};

use anyhow::{Context, Result};

use log::{debug, warn};

use std::fs::{File};
use std::io::{Write};
use self::regex::NoExpand;

pub const CARD_FRONT_FILE_PATH: &str =
    "static/templates/card_front.png";
pub const CARD_BACK_FILE_PATH: &str =
    "static/templates/card_back.png";

pub const CARD_TEMPLATE_HTML_FILE_PATH: &str =
    "static/templates/card.html";

/// Returns the path of the image it generated.
pub fn generate_image_from_html_template(
    card_info: &Card)
    -> Result<String> {
    
    let expected_image_path =
            format!("runtime/data/cards/images/{}.png", &card_info.id);
    
    let substituted_template = SingleCardTemplate::new(card_info).render()?;
    
    debug!("substituted into template: {:?}", substituted_template);
    
    
    debug!("card template file path: {:?}", CARD_TEMPLATE_HTML_FILE_PATH);
    debug!("expected image path: {:?}", expected_image_path);
    
    // Write the substituted HTML into a file
    let substituted_html_path = format!(
        "runtime/data/cards/images/templates/{}.html", &card_info.id);
    std::fs::write(&substituted_html_path, &substituted_template)?;
    
    debug!("finished writing substituted html to: {:?}",
        substituted_html_path);
    
    // Spawn off a sub-process for wkhtmltoimage to convert the image.
    generate_image_using_wkhtmltoimage(
        1050, 750, &substituted_html_path, &expected_image_path)?;
    
    // Once the image is generated, return the path to it.
    Ok(expected_image_path.to_string())
}

pub fn generate_image_using_wkhtmltoimage(
    height: u32,
    width: u32,
    substituted_html_path: &str,
    output_path: &str)
    -> Result<()> {
    
    // Spawn off a sub-process for wkhtmltoimage to convert the image.
    let child = std::process::Command::new("./wkhtmltoimage")
            .args(vec!["--height", &height.to_string(),
                "--width", &width.to_string(),
                "--enable-local-file-access",
                substituted_html_path,
                output_path])
            .output()?;
    
    if !child.status.success() {
        use crate::ServerError::FileIOError;
        Err(FileIOError(std::str::from_utf8(&child.stderr)?.to_string()))?
    }
    
    debug!("wkhtmltoimage returned success for HTML -> PNG");
    
    Ok(())
}

pub async fn retrieve_image(url: &str, card_id: i32) -> anyhow::Result<String> {
    let url = reqwest::Url::parse(url)?;
    
    debug!("parsed image url {:?}", &url);
    
    let fname = format!("runtime/data/cards/images/{:?}-art.png", card_id);
    let mut dest = File::create(&fname)?;
    
    if url.scheme() == "file" {
        let filename = &url.path()[1..];
        
        debug!("reading from local file {:?}", &filename);
        let content = std::fs::read(filename)?;
        
        debug!("writing to {:?}", fname);
        dest.write_all(&content[..])?;
    } else {
        debug!("request from url: {}", url);
    
        let response = match reqwest::get(url.clone()).await {
            Ok(res) => {
                debug!("Found successful response");
                res
            },
            Err(err) => {
                warn!("Could not get image {:?}; error: {:?}",
                    &url,
                    err);
                Err(err)?
            }
        };
    
        debug!("response: {:?}", &response);
    
        let mut content = response.bytes().await?;
        
        debug!("found content: {:?}", content);
    
        debug!("writing to {:?}", fname);
        dest.write_all(&mut content)?;
    };
    
    dest.flush()?;
    
    debug!("flushed to {:?}", fname);
    
    Ok(fname.clone())
}
