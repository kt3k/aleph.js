#[macro_use]
extern crate lazy_static;

mod css;
mod error;
mod hmr;
mod import_map;
mod jsx_magic;
mod resolve_fold;
mod resolver;
mod source_type;
mod swc;

#[cfg(test)]
mod tests;

use import_map::ImportHashMap;
use resolver::{DependencyDescriptor, Resolver};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};
use swc::{EmitOptions, SWC};
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Options {
  #[serde(default)]
  pub aleph_pkg_uri: String,

  #[serde(default)]
  pub is_dev: bool,

  #[serde(default)]
  pub import_map: ImportHashMap,

  #[serde(default)]
  pub graph_versions: HashMap<String, i64>,

  #[serde(default = "default_jsx_runtime")]
  pub jsx_runtime: String,

  #[serde(default)]
  pub jsx_runtime_version: String,

  #[serde(default)]
  pub jsx_runtime_cdn_version: String,

  #[serde(default)]
  pub jsx_import_source: String,

  #[serde(default)]
  pub jsx_magic: bool,
}

fn default_jsx_runtime() -> String {
  return "react".into();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformOutput {
  pub code: String,

  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub deps: Vec<DependencyDescriptor>,

  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub jsx_static_class_names: Vec<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub map: Option<String>,
}

#[wasm_bindgen(js_name = "transform")]
pub fn transform(specifier: &str, code: &str, options: JsValue) -> Result<JsValue, JsValue> {
  console_error_panic_hook::set_once();

  let options: Options = options
    .into_serde()
    .map_err(|err| format!("failed to parse options: {}", err))
    .unwrap();
  let resolver = Rc::new(RefCell::new(Resolver::new(
    specifier,
    &options.aleph_pkg_uri,
    &options.jsx_runtime,
    &options.jsx_runtime_version,
    &options.jsx_runtime_cdn_version,
    options.import_map,
    options.graph_versions,
    options.is_dev,
  )));
  let module = SWC::parse(specifier, code).expect("could not parse the module");
  let (code, map) = module
    .transform(
      resolver.clone(),
      &EmitOptions {
        jsx_magic: options.jsx_magic,
        jsx_import_source: options.jsx_import_source,
        minify: !options.is_dev,
        is_dev: options.is_dev,
      },
    )
    .expect("could not transform the module");
  let r = resolver.borrow();

  Ok(
    JsValue::from_serde(&TransformOutput {
      code,
      deps: r.deps.clone(),
      jsx_static_class_names: r.jsx_static_class_names.clone().into_iter().collect(),
      map,
    })
    .unwrap(),
  )
}

#[wasm_bindgen(js_name = "transformCSS")]
pub fn transform_css(filename: &str, code: &str, config_val: JsValue) -> Result<JsValue, JsValue> {
  let config: css::Config = config_val
    .into_serde()
    .map_err(|err| format!("failed to parse options: {}", err))
    .unwrap();
  let res = css::compile(filename.into(), code, &config)?;
  Ok(JsValue::from_serde(&res).unwrap())
}
