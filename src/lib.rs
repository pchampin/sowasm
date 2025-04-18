use std::{convert::Infallible, sync::Arc};
// use sophia::jsonld::loader::{BoxFuture, FutureExt};
use sophia::{
    api::{
        prelude::{Dataset, QuadParser, TripleParser},
        serializer::{QuadSerializer, Stringifier, TripleSerializer},
        source::{QuadSource, TripleSource},
        term::SimpleTerm,
    },
    inmem::dataset::LightDataset,
    iri::Iri,
    jsonld::{loader, loader_factory::LoaderFactory, JsonLdOptions},
};

mod guess_jsonld;
mod utils;
mod yamlld;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
/// Guess the syntax of the given source
pub fn guess(source: &str) -> Option<String> {
    if parser::nt::parse_str(source).works() {
        return Some("application/n-triples".into());
    }
    if parser::nq::parse_str(source).works() {
        return Some("application/n-quads".into());
    }
    let b = Iri::new_unchecked("x-string:///".to_string());
    let base = Some(b.clone());
    if (parser::turtle::TurtleParser { base })
        .parse_str(source)
        .works()
    {
        return Some("text/turtle".into());
    }
    let base = Some(b.clone());
    if (parser::trig::TriGParser { base })
        .parse_str(source)
        .works()
    {
        return Some("application/trig".into());
    }
    let base = Some(b);
    if (parser::xml::RdfXmlParser { base })
        .parse_str(source)
        .works()
    {
        return Some("application/rdf+xml".into());
    }
    // must be tested in the end, because YAML is "catching" almost anything
    if let Some(format) = guess_jsonld::guess(source) {
        return Some(format.into());
    }
    None
}

#[wasm_bindgen]
/// Convert source from one format to another
pub async fn convert(
    source: &str,
    iformat: &str,
    oformat: &str,
    base: Option<String>,
    web_doc_loader: bool,
) -> Result<String, String> {
    let lds = parse(source, iformat, base, web_doc_loader).await?;
    serialize(&lds, oformat)
}

async fn parse(
    mut source: &str,
    format: &str,
    base: Option<String>,
    web_doc_loader: bool,
) -> Result<LightDataset, String> {
    utils::set_panic_hook();
    let base = base.and_then(|s| Iri::new(s).ok());
    match format {
        "application/n-triples" => parser::nt::parse_str(source).to_lds(),
        "application/n-quads" => parser::nq::parse_str(source).to_lds(),
        "text/turtle" => {
            let par = parser::turtle::TurtleParser { base };
            par.parse_str(source).to_lds()
        }
        "application/trig" => {
            let par = parser::trig::TriGParser { base };
            par.parse_str(source).to_lds()
        }
        "application/rdf+xml" => {
            let par = parser::xml::RdfXmlParser { base };
            par.parse_str(source).to_lds()
        }
        "application/ld+json" | "application/ld+yaml" => {
            let mut buf = String::new();
            if format.ends_with("yaml") {
                source = yamlld::yaml2json(source, &mut buf)?;
            }
            let mut opt = JsonLdOptions::new();
            if let Some(base) = base {
                opt = opt.with_base(base.map_unchecked(Arc::from));
            }
            if web_doc_loader {
                parse_json_like(
                    source,
                    format,
                    opt.with_default_document_loader::<loader::HttpLoader>(),
                )
                .await
            } else {
                parse_json_like(source, format, opt).await
            }
        }
        _ => Err(format!("Unrecognized format {format}"))?,
    }
}

async fn parse_json_like<LF: LoaderFactory>(
    source: &str,
    _format: &str,
    opt: JsonLdOptions<LF>,
) -> Result<LightDataset, String> {
    let par = parser::jsonld::JsonLdParser::new_with_options(opt);
    par.async_parse_str(source).await.to_lds()
}

fn serialize(lds: &LightDataset, format: &str) -> Result<String, String> {
    utils::set_panic_hook();
    let default: Option<SimpleTerm> = None;
    let res = match format {
        "application/n-triples" => serializer::nt::NtSerializer::new_stringifier()
            .serialize_graph(&lds.graph(default))
            .map_err(|e| e.to_string())?
            .to_string(),
        "application/n-quads" => serializer::nq::NqSerializer::new_stringifier()
            .serialize_dataset(&lds)
            .map_err(|e| e.to_string())?
            .to_string(),
        "application/x-canonical-n-quads" => {
            let mut buffer: Vec<u8> = vec![];
            sophia::c14n::rdfc10::normalize(lds, &mut buffer).map_err(|e| e.to_string())?;
            String::from_utf8(buffer).map_err(|e| e.to_string())?
        }
        "text/turtle" => {
            let config = serializer::turtle::TurtleConfig::new().with_pretty(true);
            // TODO add support for prefix map ?
            serializer::turtle::TurtleSerializer::new_stringifier_with_config(config)
                .serialize_graph(&lds.graph(default))
                .map_err(|e| e.to_string())?
                .to_string()
        }
        "application/trig" => {
            let config = serializer::trig::TrigConfig::new().with_pretty(true);
            // TODO add support for prefix map ?
            serializer::trig::TrigSerializer::new_stringifier_with_config(config)
                .serialize_dataset(&lds)
                .map_err(|e| e.to_string())?
                .to_string()
        }
        "application/rdf+xml" => {
            let config = serializer::xml::RdfXmlConfig::new().with_indentation(2);
            serializer::xml::RdfXmlSerializer::new_stringifier_with_config(config)
                .serialize_graph(&lds.graph(default))
                .map_err(|e| e.to_string())?
                .to_string()
        }
        "application/ld+json" => {
            let opt = JsonLdOptions::new().with_spaces(2);
            serializer::jsonld::JsonLdSerializer::new_stringifier_with_options(opt)
                .serialize_dataset(&lds)
                .map_err(|e| e.to_string())?
                .to_string()
        }
        "application/ld+yaml" => {
            let json = serializer::jsonld::JsonLdSerializer::new_stringifier()
                .serialize_dataset(&lds)
                .map_err(|e| e.to_string())?
                .to_string();
            let mut yaml = String::new();
            yamlld::json2yaml(&json, &mut yaml)?;
            yaml
        }
        _ => Err(format!("Unrecognized format {format}"))?,
    };
    Ok(res)
}

trait TripleSourceExt: TripleSource + Sized {
    fn works(mut self) -> bool {
        self.try_for_each_triple(|_| Ok(()) as Result<(), Infallible>)
            .is_ok()
    }

    fn to_lds(self) -> Result<LightDataset, String> {
        self.to_quads().collect_quads().map_err(|e| e.to_string())
    }
}
impl<T: TripleSource + Sized> TripleSourceExt for T {}

trait QuadSourceExt: QuadSource + Sized {
    fn works(mut self) -> bool {
        self.try_for_each_quad(|_| Ok(()) as Result<(), Infallible>)
            .is_ok()
    }

    fn to_lds(self) -> Result<LightDataset, String> {
        self.collect_quads().map_err(|e| e.to_string())
    }
}
impl<T: QuadSource + Sized> QuadSourceExt for T {}

mod parser {
    pub use sophia::jsonld::parser as jsonld;
    pub use sophia::turtle::parser::{nq, nt, trig, turtle};
    pub use sophia::xml::parser as xml;
}

mod serializer {
    pub use sophia::jsonld::serializer as jsonld;
    pub use sophia::turtle::serializer::{nq, nt, trig, turtle};
    pub use sophia::xml::serializer as xml;
}
