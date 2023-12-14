use std::{convert::Infallible, sync::Arc};
// use sophia::jsonld::loader::{BoxFuture, FutureExt};
use sophia::{api::{source::{QuadSource, TripleSource}, serializer::{TripleSerializer, Stringifier, QuadSerializer}, prelude::{Dataset, TripleParser, QuadParser}, term::SimpleTerm}, inmem::dataset::LightDataset, jsonld::{JsonLdOptions, loader}, iri::Iri};

mod guess_jsonld;
mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
/// Guess the syntax of the given source
pub fn guess(source: &str) -> Option<String> {
    if parser::nt::parse_str(source).works() {
        return Some("application/n-triples".into())
    }
    if parser::nq::parse_str(source).works() {
        return Some("application/n-quads".into())
    }
    let b = Iri::new_unchecked("x-string:///".to_string());
    let base = Some(b.clone());
    if (parser::turtle::TurtleParser{ base }).parse_str(source).works() {
        return Some("text/turtle".into())
    }
    let base = Some(b.clone());
    if (parser::trig::TriGParser{ base }).parse_str(source).works() {
        return Some("application/trig".into())
    }
    if guess_jsonld::works(source) {
        return Some("application/ld+json".into())
    }
    let base = Some(b);
    if (parser::xml::RdfXmlParser { base }).parse_str(source).works() {
        return Some("application/rdf+xml".into())
    }
    None
}

#[wasm_bindgen]
/// Convert source from one format to another
pub async fn convert(source: &str, iformat: &str, oformat: &str, base: Option<String>) -> Result<String, String> {
    let lds = parse(source, iformat, base).await?;
    serialize(&lds, oformat)
}

async fn parse(source: &str, format: &str, base: Option<String>) -> Result<LightDataset, String> {
    utils::set_panic_hook();
    let base = base.and_then(|s| Iri::new(s).ok());
    match &format[..] {
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
        "application/ld+json" => {
            let mut opt = JsonLdOptions::<loader::HttpLoader>::default();
            if let Some(base) = base {
                opt = opt.with_base(base.map_unchecked(Arc::from));
            }
            let par = parser::jsonld::JsonLdParser::new_with_options(opt);
            par.async_parse_str(source).await.to_lds()
        }
        _ => Err(format!("Unrecognized format {format}"))?,
    }
}

fn serialize(lds: &LightDataset, format: &str) -> Result<String, String> {
    utils::set_panic_hook();
    let default: Option<SimpleTerm> = None;
    let res = match &format[..] {
        "application/n-triples" => serializer::nt::NtSerializer::new_stringifier()
            .serialize_graph(&lds.graph(default))
            .map_err(|e| e.to_string())?
            .to_string(),
        "application/n-quads" => serializer::nq::NqSerializer::new_stringifier()
            .serialize_dataset(&lds)
            .map_err(|e| e.to_string())?
            .to_string(),
        "application/canonical-n-quads" => {
            let mut buffer: Vec<u8> = vec![];
            sophia::c14n::rdfc10::normalize(&lds, &mut buffer).map_err(|e| e.to_string())?;
            String::from_utf8(buffer).map_err(|e| e.to_string())?
        }
        "text/turtle" => {
            let config = serializer::turtle::TurtleConfig::new()
                .with_pretty(true);
                // TODO add support for prefix map ?
            serializer::turtle::TurtleSerializer::new_stringifier_with_config(config)
                .serialize_graph(&lds.graph(default))
                .map_err(|e| e.to_string())?
                .to_string()
        }
        "application/trig" => {
            let config = serializer::trig::TrigConfig::new()
                .with_pretty(true);
                // TODO add support for prefix map ?
            serializer::trig::TrigSerializer::new_stringifier_with_config(config)
                .serialize_dataset(&lds)
                .map_err(|e| e.to_string())?
                .to_string()
        }
        "application/rdf+xml" => {
            let config = serializer::xml::RdfXmlConfig{};
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
        _ => Err(format!("Unrecognized format {format}"))?,
    };
    Ok(res)
}

trait TripleSourceExt: TripleSource + Sized {
    fn works(mut self) -> bool {
        self.try_for_each_triple(|_| Ok(()) as  Result<(), Infallible>).is_ok()
    }

    fn how_many(mut self) -> Result<usize, usize> {
        loop {
            let mut c = 0;
            match self.try_for_some_triple(|_| -> Result<(), Infallible> {
                c += 1;
                Ok(())
            }) {
                Err(_) => return Err(c),
                Ok(false) => return Ok(c),
                Ok(true) => {}
            }
        }
    }

    fn to_lds(self) -> Result<LightDataset, String> {
        self.to_quads().collect_quads().map_err(|e| e.to_string())
    }
}
impl <T: TripleSource + Sized> TripleSourceExt for T {}

trait QuadSourceExt: QuadSource + Sized {
    fn works(mut self) -> bool {
        self.try_for_each_quad(|_| Ok(()) as  Result<(), Infallible>).is_ok()
    }

    fn how_many(mut self) -> Result<usize, usize> {
        loop {
            let mut c = 0;
            match self.try_for_some_quad(|_| -> Result<(), Infallible> {
                c += 1;
                Ok(())
            }) {
                Err(_) => return Err(c),
                Ok(false) => return Ok(c),
                Ok(true) => {}
            }
        }
    }

    fn to_lds(self) -> Result<LightDataset, String> {
        self.collect_quads().map_err(|e| e.to_string())
    }
}
impl <T: QuadSource + Sized> QuadSourceExt for T {}

mod parser {
    pub use sophia::turtle::parser::{nq, nt, trig, turtle};
    pub use sophia::xml::parser as xml;
    pub use sophia::jsonld::parser as jsonld;
}

mod serializer {
    pub use sophia::turtle::serializer::{nq, nt, trig, turtle};
    pub use sophia::xml::serializer as xml;
    pub use sophia::jsonld::serializer as jsonld;
}
