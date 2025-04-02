use serde_json::{json, Value};

use alloy::primitives::Address;
use axum::{
    extract::{Multipart, Query, State},
    Json,
};
use serde::Deserialize;
use sqlx::{postgres::PgRow, Row};

use anyhow::anyhow;
use std::sync::Arc;
use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Deserialize)]
pub struct PortfolioListRequest {
    to: i64,
    countback: i64,
    resolution: String,
    multipool_address: Address,
    chain_id: i64,
}

// TODO: portfolio list
pub async fn list(
    Query(query): Query<PortfolioListRequest>,
    State(client): State<Arc<sqlx::PgPool>>,
) -> Json<Value> {
    let to = &query.to;
    let countback = query.countback;
    let resolution: i32 = if query.resolution == "1D" {
        1440 * 60
    } else {
        let parsed_number: Result<i32, _> = query.resolution.parse();
        match parsed_number {
            Ok(num) => num * 60,
            Err(err) => return json!({"err":err.to_string()}).into(),
        }
    };
    let result = sqlx::query(
        "
        SELECT 
            open::TEXT as o, 
            close::TEXT as c, 
            low::TEXT as l, 
            high::TEXT as h, 
            ts::TEXT as t
        FROM 
            candles
        WHERE 
            ts <= $1
            AND resolution = $2
            AND multipool = $3
            AND chain_id = $3
        ORDER BY 
            ts DESC
        LIMIT $4;",
    )
    .bind(to)
    .bind(resolution)
    .bind::<&[u8]>(query.multipool_address.as_slice())
    .bind(countback)
    .bind(query.chain_id)
    .fetch_all(client.as_ref())
    .await;

    match result {
        Ok(rows) => {
            if rows.is_empty() {
                json!({"s": "no_data"}).into()
            } else {
                json!({
                    "s":"ok",
                    "t": rows.iter().rev().map(|r: &PgRow| r.get("t")).collect::<Vec<String>>(),
                    "o": rows.iter().rev().map(|r: &PgRow| r.get("o") ).collect::<Vec<String>>(),
                    "c": rows.iter().rev().map(|r: &PgRow| r.get("c") ).collect::<Vec<String>>(),
                    "l": rows.iter().rev().map(|r: &PgRow| r.get("l") ).collect::<Vec<String>>(),
                    "h": rows.iter().rev().map(|r: &PgRow| r.get("h") ).collect::<Vec<String>>(),
                })
                .into()
            }
        }
        Err(err) => {
            println!("{:?}", err);
            json!({"s":"error"}).into()
        }
    }
}

#[derive(Deserialize)]
struct FormData {
    multipool_address: Address,
    chain_id: i64,
    #[serde(skip)] // Skip for file data
    logo: Vec<u8>,
    symol: String,
    name: String,
    description: String,
}

fn stringify<E: ToString>(e: E) -> String {
    e.to_string()
}

pub async fn create(
    State(client): State<Arc<sqlx::PgPool>>,
    mut multipart: Multipart,
) -> Result<Json<Value>, String> {
    let mut logo = None;
    let mut multipool_address: Option<Address> = None;
    let mut chain_id: Option<i64> = None;
    let mut symbol: Option<_> = None;
    let mut name: Option<_> = None;
    let mut description: Option<_> = None;

    while let Some(mut field) = multipart.next_field().await.map_err(stringify)? {
        let field_name = field.name().unwrap_or_default().to_string();

        if field_name == "logo" {
            let mut file_data = Vec::new();
            while let Some(chunk) = field.chunk().await.map_err(stringify)? {
                file_data.extend_from_slice(&chunk);
            }
            logo = Some(file_data);
        } else if field_name == "name" {
            name = Some(field.text().await.map_err(stringify)?);
        } else if field_name == "chain_id" {
            chain_id = Some(
                field
                    .text()
                    .await
                    .map_err(stringify)?
                    .parse()
                    .map_err(stringify)?,
            );
        } else if field_name == "symbol" {
            symbol = Some(field.text().await.map_err(stringify)?);
        } else if field_name == "description" {
            description = Some(field.text().await.map_err(stringify)?);
        } else if field_name == "multipool_address" {
            multipool_address = Some(
                field
                    .text()
                    .await
                    .map_err(stringify)?
                    .parse()
                    .map_err(stringify)?,
            );
        }
    }

    let logo = logo.ok_or(anyhow!("Invalid form")).map_err(stringify)?;
    let multipool_address = multipool_address
        .ok_or(anyhow!("Invalid form"))
        .map_err(stringify)?;
    let chain_id = chain_id.ok_or(anyhow!("Invalid form")).map_err(stringify)?;
    let symbol = symbol.ok_or(anyhow!("Invalid form")).map_err(stringify)?;
    let name = name.ok_or(anyhow!("Invalid form")).map_err(stringify)?;
    let description = description
        .ok_or(anyhow!("Invalid form"))
        .map_err(stringify)?;
    //TODO: add limits on name, symbol, description + logo size

    let file_path = format!("/logos/{multipool_address}");
    let mut file_handle = File::create(file_path).await.map_err(stringify)?;
    file_handle.write_all(&logo).await.map_err(stringify)?;

    sqlx::query("INSERT INTO multipools(chain_id, multipool, name, symbol, description, logo) VALUES($1,$2,$3,$4,$5,$6);")
        .bind(chain_id)
        .bind::<&[u8]>(multipool_address.as_slice())
        .bind(name)
        .bind(symbol)
        .bind(description)
        .execute(client.as_ref())
        .await.map_err(stringify)
        .map(|_| json!(()).into())
}

// TODO portfolio info from storage
pub async fn portfolio(State(_client): State<Arc<sqlx::PgPool>>) -> Json<Value> {
    json!(()).into()
}
