use std::{collections::HashMap, env};

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
//use tokio::{sync::RwLock, time::sleep};
use tokio_postgres::{NoTls,Client};

enum ApiErrors {
    ResolutionOverflow,
}

#[derive(Deserialize)]
pub struct SymbolRequest {
    symbol: String,
}

#[derive(Deserialize)]
pub struct HistoryRequest {
    to: String,
    countback: u128,
    resolution: String,
    symbol: String
}

#[derive(Deserialize)]
pub struct StatsRequest {
    multipool_id: String,
}

#[get("/api/tv/config")]
async fn config() -> impl Responder {
    HttpResponse::Ok()
        .json(json!({
        "supported_resolutions": ["1", "3", "5", "15", "30", "60", "720", "1D"],
        "has_intraday": true,
        "supports_group_request": false,
        "supports_marks": false,
        "supports_search": true,
        "supports_timescale_marks": false,
    }))
}

#[get("/api/tv/symbols")]
async fn symbols(query_params: web::Query<SymbolRequest>) -> impl Responder {
    let symbol = &query_params.symbol;
    HttpResponse::Ok().json(json!({
        "description": "Description",
        "supported_resolutions": ["1", "3", "5", "15", "30", "60", "720", "1D"],
        "exchange": "no",
        "full_name": symbol,
        "name": symbol,
        "symbol": symbol,
        "ticker": symbol,
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

#[get("/api/tv/history")]
async fn history(query_params: web::Query<HistoryRequest>,client:web::Data<Client>) -> HttpResponse {
    let to = &query_params.to;
    let symbol = &query_params.symbol;
    let countback = query_params.countback;
    let resolution: i64;
    if query_params.resolution == "1D" {
        resolution = 1440 * 60
    } else {
        let parsed_number: Result<i64,_> = query_params.resolution.parse();
        resolution = match parsed_number {
            Ok(num) => num * 60,
            Err(err) => return HttpResponse::Ok().json(json!({"err":err.to_string()}))
        };
    }
    let query = format!(
            "SELECT open as o, close as c, low as l, high as h, ts as t
            FROM candles
            WHERE ts <= $1
            AND resolution = $2
            AND multipool_id = $3
            ORDER BY ts DESC
            LIMIT $4;"
        );
    let result = client.query(query.as_str(), &[&to, &resolution, &symbol, &(countback as i64)]).await;
    match result {
            Ok(rows) => {
                if rows.len() == 0 {
                    return HttpResponse::Ok().json(json!({"s": "no_data"}))
                }
                let mut o_vec: Vec<u32> = vec![];
                let mut c_vec: Vec<f64> = vec![];
                let mut l_vec: Vec<f64> = vec![];
                let mut h_vec: Vec<f64> = vec![];
                let mut t_vec: Vec<f64> = vec![];
                rows.iter().for_each(|row| {
                    o_vec.push(row.get("o"));
                    c_vec.push(row.get("c"));
                    l_vec.push(row.get("l"));
                    h_vec.push(row.get("h"));
                    t_vec.push(row.get("t"));
                });
                HttpResponse::Ok().json(json!({"s":"ok", "t": t_vec,"o": o_vec, "c": c_vec, "l": l_vec, "h": h_vec }))
            },
            Err(err) => {
                println!("{:?}",err);
                HttpResponse::Ok().json(json!({"s":"error"}))
            }
        }
}

#[get("/api/stats")]
async fn stats(
    query_params: web::Query<StatsRequest>,
    client: web::Data<Client>,
) -> HttpResponse {
        let multipool_id = &query_params.multipool_id;
        let query = "
                SELECT *
                FROM multipools
                WHERE multipool_id = $1;
            ";
        let result = client
            .query(query, &[&multipool_id.to_lowercase()])
            .await;
        match result {
            Ok(rows) => {
                if let Some(row) = rows.first() {
                    let mp_id:String = row.get("multipool_id");
                    let change_24h:String = row.get("change_24h");
                    let low_24h:String = row.get("low_24h");
                    let high_24h:String = row.get("high_24h");
                    let current_price:String = row.get("current_price");
                    return HttpResponse::Ok().json(json!({"multipool_id":mp_id,"change_24h":change_24h,"low_24h":low_24h,"high_24h":high_24h,"current_price":current_price}))
                } else {
                    return HttpResponse::Ok().json(json!({"err":"no_data"}))
                }
            }
            Err(err) => {
                return HttpResponse::Ok().json(json!({"err":err.to_string()}))
            }
        }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let bind_address = env::var("BIND_ADDRESS").unwrap_or("0.0.0.0:8080".into());
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("{:?},{:?}",database_url,bind_address);
    let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
        .await
        .expect("Postres connect should be valid");
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
            std::process::exit(0x0700);
        }
    });
    HttpServer::new(|| {
            App::new()
                .service(config)
                .service(symbols)
                .service(history)
                .service(stats)
        })
        .bind(bind_address)?
        .run()
        .await
}
