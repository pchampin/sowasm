//! This is a dummy parser that does not fail on loading remote contexts,
//! only to check if the source is valid JSON-LD.
//!
#![allow(dead_code, unused)]

use sophia::api::parser::QuadParser;
use sophia::iri::Iri;
use sophia::jsonld::{loader::*, JsonLdOptions, JsonLdParser};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

use super::*;

pub fn guess(source: &str) -> Option<&'static str> {
    let opts = JsonLdOptions::new()
        .with_base(Iri::new_unchecked("x-string:///".into()))
        .with_document_loader_closure(|| {
            ClosureLoader::new(|iri: Iri<String>| -> BoxFuture<Result<String, String>> {
                let url = iri.as_str().to_string();
                async move { Ok(format!(r#"{{ "@context": {{ "@vocab": "{url}#" }} }}"#)) }.boxed()
            })
        });
    let p = JsonLdParser::new_with_options(opts);
    if p.parse_str(source).works() {
        return Some("application/ld+json");
    }
    let mut buf = String::new();
    if let Ok(converted) = yamlld::yaml2json(source, &mut buf) {
        if p.parse_str(converted).works() {
            return Some("application/ld+yaml");
        }
    }
    None
}
