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
    options: SwaggerOptions,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SwaggerOptions {
    /// initially collapse the tag groups
    pub collapse: bool,
}

/// Provide an override configuration for the swagger initialization
#[get("/swagger-initializer.js")]
async fn config(config: web::Data<SwaggerConfig>) -> impl Responder {
    let expansion = match config.options.collapse {
        true => r#"docExpansion: "none","#,
        false => "",
    };

    format!(
        r#"
window.onload = function() {{
    const options = {{
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
        {expansion}
        layout: "StandaloneLayout"
    }};
    window.ui = SwaggerUIBundle(options);
}};
    "#,
        url = config.api_url,
    )
}

pub fn service(
    api_url: &str,
    options: impl Into<Option<SwaggerOptions>>,
) -> impl HttpServiceFactory {
    let c = SwaggerConfig {
        api_url: api_url.into(),
        options: options.into().unwrap_or_default(),
    };

    web::scope("")
        .app_data(web::Data::new(c))
        .service(config)
        .service(ResourceFiles::new("/", assets()))
}
