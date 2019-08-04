use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Result};
use futures::{future::ok, Future};

use std::net::SocketAddr;

use maxminddb::geoip2::City;
use maxminddb::MaxMindDBError;
use maxminddb::Reader;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;

use serde_json;

#[derive(Serialize)]
struct NonResolvedIPResponse<'a> {
    pub ip_address: &'a str,
}
#[derive(Serialize, Deserialize)]
struct ResolvedIPResponse<'a> {
    pub country_name: &'a str,
    pub city_name: &'a str,
}
fn index_async(req: HttpRequest) -> impl Future<Item = HttpResponse, Error = Error> {
    println!("{:?}", req);
    let language = String::from("zh-CN");
    let server: SocketAddr = req
        .connection_info()
        .remote()
        .unwrap()
        .parse()
        .expect("Unable to parse socket address");

    /// ```shell
    /// wget https://geolite.maxmind.com/download/geoip/database/GeoLite2-City.tar.gz
    /// tar xf GeoLite2-City.tar.gz -C /root
    ///```
    let db = Arc::new(Reader::open_mmap(db_file_path()).unwrap());

    let lookup: Result<City, MaxMindDBError> = db.lookup(server.ip());

    let geoip = match lookup {
        Ok(geoip) => {
            let res = ResolvedIPResponse {
                country_name: geoip
                    .country
                    .as_ref()
                    .and_then(|country| country.names.as_ref())
                    .and_then(|names| names.get(&language))
                    .map(String::as_str)
                    .unwrap_or(""),
                city_name: geoip
                    .city
                    .as_ref()
                    .and_then(|city| city.names.as_ref())
                    .and_then(|names| names.get(&language))
                    .map(String::as_str)
                    .unwrap_or(""),
            };
            println!("{}, {}", res.country_name, res.city_name);
            serde_json::to_string(&res).ok()
        }
        Err(_) => serde_json::to_string(&NonResolvedIPResponse {
            ip_address: "1.1.1.1",
        })
        .ok(),
    }
    .unwrap();
    println!("{:?}", geoip);
    ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(format!("{}\t{:?}\n", server.ip(), geoip)))
}

/// 404 handler
fn p404() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body("<a href=/ip>IP</a>".to_string()))
}

fn db_file_path() -> String {
    env::var("GEOIP_RS_DB_PATH")
        .unwrap_or_else(|_| String::from("/root/GeoLite2-City_20190730/GeoLite2-City.mmdb"))
    //
    //    let args: Vec<String> = env::args().collect();
    //    if args.len() > 1 {
    //        return args[1].to_string();
    //    }
    //    panic!("You must specify the db path, either as a command line argument or as GEOIP_RS_DB_PATH env var");
}

fn main() -> std::io::Result<()> {
    let sys = actix_rt::System::new("basic-example");
    HttpServer::new(|| {
        App::new()
            .service(web::resource("/ip").route(web::get().to_async(index_async)))
            .default_service(web::resource("").route(web::get().to(p404)))
    })
    .bind("0.0.0.0:8080")?
    .start();

    println!("Starting http server: http://0.0.0.0:8080");
    sys.run()
}
