use seedwing_policy_engine::runtime::{PackagePath, TypeName};
use serde::Serialize;

#[derive(Serialize)]
pub struct Breadcrumbs {
    crumbs: Vec<Breadcrumb>,
}

#[derive(Serialize)]
pub struct Breadcrumb {
    text: String,
    link: Option<String>,
}

impl From<(TypeName, Vec<String>)> for Breadcrumbs {
    fn from((name, params): (TypeName, Vec<String>)) -> Self {
        let segments = name.segments();

        let mut crumbs = Vec::new();

        let mut upwalk: String = "./".into();

        for (i, segment) in segments.iter().rev().enumerate() {
            if i == 0 {
                let text = if params.is_empty() {
                    segment.clone()
                } else {
                    let mut tmp = segment.clone();
                    tmp.push('<');
                    tmp.push_str(params.join(", ").as_str());
                    tmp.push('>');
                    tmp
                };
                crumbs.push(Breadcrumb { text, link: None })
            } else {
                crumbs.push(Breadcrumb {
                    text: segment.clone(),
                    link: Some(upwalk.clone()),
                });
                upwalk.push_str("../");
            }
        }

        crumbs.reverse();

        Self { crumbs }
    }
}

impl From<PackagePath> for Breadcrumbs {
    fn from(path: PackagePath) -> Self {
        let segments = path.segments();

        let mut crumbs = Vec::new();

        let mut upwalk: String = "../".into();

        for (i, segment) in segments.iter().rev().enumerate() {
            if i == 0 {
                crumbs.push(Breadcrumb {
                    text: segment.clone(),
                    link: None,
                })
            } else {
                crumbs.push(Breadcrumb {
                    text: segment.clone(),
                    link: Some(upwalk.clone()),
                });
                upwalk.push_str("../");
            }
        }

        crumbs.reverse();

        Self { crumbs }
    }
}
