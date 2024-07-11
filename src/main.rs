use reqwest::Client;
use serde_json::Value;
use std::env;
use dotenv::dotenv;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let notion_token = env::var("NOTION_TOKEN").expect("NOTION_TOKEN must be set");
    let page_id = env::var("NOTION_PAGE_ID").expect("NOTION_PAGE_ID must be set");

    let client = Client::new();

    iterate_blocks(&client, &page_id, &notion_token).await?;

    println!("Todos los bloques han sido procesados.");

    Ok(())
}

async fn get_page_content(client: &Client, page_id: &str, notion_token: &str) -> Result<Vec<Value>, reqwest::Error> {
    let url = format!("https://api.notion.com/v1/blocks/{}/children", page_id);
    let res = client.get(&url)
        .header("Authorization", format!("Bearer {}", notion_token))
    .header("Notion-Version", "2022-06-28")
        .send().await?
    .json::<Value>()
        .await?;

    Ok(res["results"].as_array().unwrap_or_else(|| {
        panic!("Expected 'results' array in Notion API response");
    }).to_vec())
}

async fn update_checkbox(client: &Client, block_id: &str, checked: bool, notion_token: &str) -> Result<(), reqwest::Error> {
    let url = format!("https://api.notion.com/v1/blocks/{}", block_id);
    let body = serde_json::json!({
        "to_do": {
            "checked": checked
        }
    });

    client.patch(&url)
        .header("Authorization", format!("Bearer {}", notion_token))
        .header("Notion-Version", "2022-06-28")
        .json(&body)
        .send()
        .await?;

    Ok(())
}

async fn process_block(client: &Client, block: &Value, notion_token: &str) -> Result<(), Box<dyn Error>> {
    if block["has_children"].as_bool().unwrap_or(false) {
        let children = get_page_content(client, block["id"].as_str().unwrap(), notion_token).await?;
        for child in children {
            let fut = Box::pin(process_block(client, &child, notion_token));
            fut.await?;
        }
    } else if block["type"] == "to_do" && block["to_do"]["checked"].as_bool().unwrap_or(false) {
        update_checkbox(client, block["id"].as_str().unwrap(), false, notion_token).await?;
    }
    Ok(())
}

async fn iterate_blocks(client: &Client, page_id: &str, notion_token: &str) -> Result<(), Box<dyn Error>> {
    let blocks = get_page_content(client, page_id, notion_token).await?;
    for block in blocks {
        process_block(client, &block, notion_token).await?;
    }
    Ok(())
}
