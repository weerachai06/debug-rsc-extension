//! JSON-string shell over `Session`. serde_json matches what ts-rs describes; serde-wasm-bindgen does not.
//! Bad input JSON is a programmer error and traps.

use wasm_bindgen::prelude::*;

use crate::{Chunk, Loc, Query, Session};

#[wasm_bindgen]
pub struct Flight(Session);

#[wasm_bindgen]
impl Flight {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Flight {
        Flight(Session::default())
    }

    pub fn feed(&mut self, chunk_json: &str) {
        self.0.feed(serde_json::from_str::<Chunk>(chunk_json).expect("invalid Chunk JSON"));
    }

    pub fn overview(&self) -> String {
        serde_json::to_string(&self.0.overview()).expect("Overview serializes")
    }

    pub fn children(&self, loc_json: &str) -> String {
        let at = serde_json::from_str::<Loc>(loc_json).expect("invalid Loc JSON");
        serde_json::to_string(&self.0.children(&at)).expect("Nodes serialize")
    }

    pub fn search(&self, query_json: &str) -> String {
        let query = serde_json::from_str::<Query>(query_json).expect("invalid Query JSON");
        serde_json::to_string(&self.0.search(&query)).expect("Hits serialize")
    }
}
