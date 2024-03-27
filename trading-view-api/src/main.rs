use std::env;
use std::sync::Arc;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

use serde::Deserialize;
use serde_json::json;
use tokio_postgres::{Client, NoTls};

use actix_cors::Cors;

use ethers::prelude::*;

#[derive(Deserialize)]
pub struct SymbolRequest {
    symbol: String,
}

#[derive(Deserialize)]
pub struct HistoryRequest {
    to: i64,
    countback: i64,
    resolution: String,
    symbol: Address,
}

#[derive(Deserialize)]
pub struct StatsRequest {
    multipool_id: Address,
}

#[get("/config")]
async fn config() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "supported_resolutions": ["1", "3", "5", "15", "30", "60", "720", "1D"],
        "has_intraday": true,
        "supports_group_request": false,
        "supports_marks": false,
        "supports_search": true,
        "supports_timescale_marks": false,
    }))
}

#[get("/symbols")]
async fn symbols(query_params: web::Query<SymbolRequest>) -> impl Responder {
    let symbol = &query_params.symbol;
    HttpResponse::Ok().json(json!({
        "description": " ",
        "supported_resolutions": ["1", "3", "5", "15", "30", "60", "720", "1D"],
        "exchange": "no",
        "full_name": symbol.to_string(),
        "name": symbol.to_string(),
        "symbol": symbol.to_string(),
        "ticker": symbol.to_string(),
        "type": "Spot",
        "session": "24x7",
        "listed_exchange": "no",
        "timezone": "Etc/UTC",
        "has_intraday": true,
        "minmov": 1,
        "pricescale": 1000,
    }))
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/history")]
async fn history(
    query_params: web::Query<HistoryRequest>,
    client: web::Data<Arc<Client>>,
) -> HttpResponse {
    let to = &query_params.to;
    let symbol = &query_params.symbol;
    let countback = query_params.countback;
    let resolution: i32 = if query_params.resolution == "1D" {
        1440 * 60
    } else {
        let parsed_number: Result<i32, _> = query_params.resolution.parse();
        match parsed_number {
            Ok(num) => num * 60,
            Err(err) => return HttpResponse::Ok().json(json!({"err":err.to_string()})),
        }
    };
    let query = "
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
            AND multipool_id = $3
        ORDER BY 
            ts DESC
        LIMIT $4;"
        .to_string();
    let result = client
        .query(
            query.as_str(),
            &[
                &to,
                &resolution,
                &serde_json::to_string(&symbol).unwrap().trim_matches('\"'),
                &countback,
            ],
        )
        .await;
    match result {
        Ok(rows) => {
            if rows.is_empty() {
                HttpResponse::Ok().json(json!({"s": "no_data"}))
            } else {
                HttpResponse::Ok().json(json!({
                    "s":"ok",
                    "t": rows.iter().rev().map(|r| r.get("t") ).collect::<Vec<String>>(),
                    "o": rows.iter().rev().map(|r| r.get("o") ).collect::<Vec<String>>(),
                    "c": rows.iter().rev().map(|r| r.get("c") ).collect::<Vec<String>>(),
                    "l": rows.iter().rev().map(|r| r.get("l") ).collect::<Vec<String>>(),
                    "h": rows.iter().rev().map(|r| r.get("h") ).collect::<Vec<String>>(),
                }))
            }
        }
        Err(err) => {
            println!("{:?}", err);
            HttpResponse::Ok().json(json!({"s":"error"}))
        }
    }
}

#[get("/stats")]
async fn stats(
    query_params: web::Query<StatsRequest>,
    client: web::Data<Arc<Client>>,
) -> HttpResponse {
    let multipool_id = &query_params.multipool_id;
    let query = "
                SELECT 
                    multipool_id,
                    change_24h::TEXT,
                    low_24h::TEXT,
                    high_24h::TEXT,
                    current_price::TEXT
                FROM multipools
                WHERE multipool_id = $1;
            ";
    let result = client
        .query(
            query,
            &[&serde_json::to_string(&multipool_id)
                .unwrap()
                .trim_matches('\"')],
        )
        .await;
    match result {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                let mp_id: String = row.get("multipool_id");
                let change_24h: String = row.get("change_24h");
                let low_24h: String = row.get("low_24h");
                let high_24h: String = row.get("high_24h");
                let current_price: String = row.get("current_price");
                HttpResponse::Ok().json(json!({"multipool_id":mp_id,"change_24h":change_24h,"low_24h":low_24h,"high_24h":high_24h,"current_price":current_price}))
            } else {
                HttpResponse::Ok().json(json!({"err":"no_data"}))
            }
        }
        Err(err) => HttpResponse::Ok().json(json!({"err":err.to_string()})),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let bind_address = env::var("BIND_ADDRESS").unwrap_or("0.0.0.0:8080".into());
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
        .await
        .expect("Postres connect should be valid");
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            println!("connection error: {}", e);
            std::process::exit(0x0700);
        }
    });
    let client = Arc::new(client);
    HttpServer::new(move || {
        let cors = Cors::permissive();
        let client = client.clone();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(client))
            .service(config)
            .service(symbols)
            .service(history)
            .service(stats)
    })
    .bind(bind_address)?
    .run()
    .await
}
