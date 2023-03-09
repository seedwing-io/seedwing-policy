use actix_web::dev::HttpServiceFactory;
use actix_web::{get, web, Responder};
use actix_web_static_files::ResourceFiles;
use std::collections::HashMap;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

pub fn assets() -> HashMap<&'static str, static_files::Resource> {
    generate_assets()
}

#[derive(Clone, Debug)]
pub struct SwaggerConfig {
    api_url: String,
}

/// Provide an override configuration for the swagger initialization
#[get("/swagger-initializer.js")]
async fn config(config: web::Data<SwaggerConfig>) -> impl Responder {
    format!(
        r#"
window.onload = function() {{
  //<editor-fold desc="Changeable Configuration Block">

  // the following lines will be replaced by docker/configurator, when it runs in a docker-container
  window.ui = SwaggerUIBundle({{
    url: "{url}",
    dom_id: '#swagger-ui',
    deepLinking: true,
    presets: [
      SwaggerUIBundle.presets.apis,
      SwaggerUIStandalonePreset
    ],
    plugins: [
      SwaggerUIBundle.plugins.DownloadUrl
    ],
    layout: "StandaloneLayout"
  }});

  //</editor-fold>
}};
    "#,
        url = config.api_url,
    )
}

pub fn service(api_url: &str) -> impl HttpServiceFactory {
    let c = SwaggerConfig {
        api_url: api_url.into(),
    };

    web::scope("")
        .app_data(web::Data::new(c))
        .service(config)
        .service(ResourceFiles::new("/", assets()))
}
