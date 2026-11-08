//! Test tooling only: inspect Rust syntax across every Cargo workspace member.
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
    process::Command,
};
use syn::visit::{self, Visit};

const BOUNDARIES: &[&str] = &[
    "chat_stream",
    "stream_complete_llm",
    "stream_complete_llm_with_sink",
    "chat_json_stream_result_with_sink",
    "execute_llm_generation",
    "execute_validated_llm_generation",
    "execute_outline_llm_call",
    "execute_portrait_llm_call",
    "execute_task_llm_chat_response",
    "run_workflow_llm_once",
    "render_and_chat_trace_with_context",
    "generate_image",
    "generate_video",
    "embed",
    "embed_with_identity",
    "test_connection",
    "start_job",
    "poll_job",
    "job_status",
    "delete_job",
    "generate",
    "start_generation",
    "poll_generation",
    "collect_chat_stream",
    "poll_video",
    "video_status",
    "delete_video",
    "poll",
    "status",
    "delete",
];

fn is_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
            || (attr.path().is_ident("cfg")
                && attr
                    .parse_args::<syn::Meta>()
                    .is_ok_and(|meta| requires_test(&meta)))
    })
}

fn requires_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) => {
            use syn::parse::Parser;
            let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
            let Ok(children) = parser.parse2(list.tokens.clone()) else {
                return false;
            };
            if list.path.is_ident("all") {
                children.iter().any(requires_test)
            } else {
                list.path.is_ident("any")
                    && !children.is_empty()
                    && children.iter().all(requires_test)
            }
        }
        _ => false,
    }
}

#[derive(Default)]
struct Calls {
    scope: Vec<String>,
    rows: Vec<Value>,
    ordinals: BTreeMap<(String, String), usize>,
    implementations: Vec<String>,
}

impl Calls {
    fn record(&mut self, callee: &str, receiver: Option<String>) {
        let function = self.scope.join("::");
        let ordinal = self
            .ordinals
            .entry((function.clone(), callee.into()))
            .or_default();
        *ordinal += 1;
        self.rows.push(
            json!({"function":function,"callee":callee,"ordinal":ordinal,"receiver":receiver}),
        );
    }
}

fn discover(rows: Vec<Value>) -> Vec<Value> {
    let mut names = BOUNDARIES
        .iter()
        .map(|name| name.to_string())
        .collect::<std::collections::BTreeSet<_>>();
    let generic = [
        "new",
        "run",
        "execute",
        "handle",
        "dispatch",
        "call",
        "start",
        "hash_password",
        "get",
        "list",
        "from",
        "try_from",
        "serialize",
        "deserialize",
        "as_ref",
        "clone",
        "default",
        "to_string",
        "fmt",
        "validate",
        "finish",
        "push",
        "append",
        "send",
        "collect",
        "next",
        "build",
        "record",
        "render_prompt",
        "render_prompt_for_call",
        "request_body",
    ];
    let mut definitions: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for row in &rows {
        let function = row["function"].as_str().unwrap();
        let name = function.rsplit("::").next().unwrap();
        definitions.entry(name.into()).or_default().insert(format!(
            "{}::{function}",
            row.get("file").and_then(Value::as_str).unwrap_or("test")
        ));
    }
    loop {
        let before = names.len();
        for row in &rows {
            let callee = row["callee"].as_str().expect("callee");
            let function = row["function"]
                .as_str()
                .expect("function")
                .rsplit("::")
                .next()
                .unwrap();
            if names.contains(callee)
                && !generic.contains(&function)
                && !function.is_empty()
                && definitions
                    .get(function)
                    .is_some_and(|definitions| definitions.len() == 1)
            {
                names.insert(function.to_string());
            }
        }
        if names.len() == before {
            break;
        }
    }
    rows.into_iter()
        .filter(|row| names.contains(row["callee"].as_str().expect("callee")))
        .collect()
}

impl<'ast> Visit<'ast> for Calls {
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if is_test(&item.attrs) {
            return;
        }
        if let Some((_, path, _)) = &item.trait_
            && let Some(name) = path.segments.last()
            && [
                "LlmProvider",
                "ImageProvider",
                "VideoProvider",
                "EmbeddingProvider",
            ]
            .contains(&name.ident.to_string().as_str())
        {
            self.implementations.push(name.ident.to_string());
        }
        visit::visit_item_impl(self, item);
    }
    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if is_test(&item.attrs) {
            return;
        }
        self.scope.push(item.sig.ident.to_string());
        visit::visit_trait_item_fn(self, item);
        self.scope.pop();
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if is_test(&item.attrs) {
            return;
        }
        self.scope.push(item.ident.to_string());
        visit::visit_item_mod(self, item);
        self.scope.pop();
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if is_test(&item.attrs) {
            return;
        }
        self.scope.push(item.sig.ident.to_string());
        visit::visit_item_fn(self, item);
        self.scope.pop();
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if is_test(&item.attrs) {
            return;
        }
        self.scope.push(item.sig.ident.to_string());
        visit::visit_impl_item_fn(self, item);
        self.scope.pop();
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let receiver = match call.receiver.as_ref() {
            syn::Expr::Path(path) => path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string()),
            syn::Expr::Field(field) => match &field.member {
                syn::Member::Named(name) => Some(name.to_string()),
                _ => None,
            },
            _ => None,
        };
        let method = call.method.to_string();
        if !["poll", "status", "delete"].contains(&method.as_str())
            || receiver
                .as_deref()
                .is_some_and(|name| ["provider", "video", "video_client", "inner"].contains(&name))
        {
            self.record(&method, receiver);
        }
        visit::visit_expr_method_call(self, call);
    }
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref()
            && let Some(segment) = path.path.segments.last()
        {
            self.record(&segment.ident.to_string(), None);
        }
        visit::visit_expr_call(self, call);
    }
}

fn source_files(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            if path.file_name().is_some_and(|name| name == "tests") {
                continue;
            }
            source_files(&path, output)?;
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && path.file_name().is_some_and(|name| name != "tests.rs")
        {
            output.push(path);
        }
    }
    Ok(())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("validate") {
        let mut validators = BTreeMap::new();
        for line in io::stdin().lock().lines() {
            let record: Value = serde_json::from_str(&line?)?;
            if record["kind"] == "schema" {
                let id = record["schema_id"].as_str().ok_or("schema id missing")?;
                if validators
                    .insert(
                        id.to_string(),
                        a4n_domain::json_contract::OutputSchemaValidator::new(&record["schema"])?,
                    )
                    .is_some()
                {
                    return Err("duplicate schema definition".into());
                }
                continue;
            }
            let id = record["schema_id"]
                .as_str()
                .ok_or("case schema id missing")?;
            let schema = validators
                .get(id)
                .ok_or("case references undefined schema")?;
            let result = schema.validate(&record["value"]);
            println!(
                "{}",
                json!({"case_id":record["case_id"],"accepted":result.is_ok(),"error":result.err()})
            );
        }
        return Ok(());
    }
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?)
        .canonicalize()?;
    let metadata = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(&root)
        .output()?;
    if !metadata.status.success() {
        return Err("cargo metadata failed".into());
    }
    let metadata: Value = serde_json::from_slice(&metadata.stdout)?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("workspace members missing")?;
    let mut rows = Vec::new();
    let mut implementations = Vec::new();
    let mut dependencies = BTreeMap::new();
    for package in metadata["packages"].as_array().ok_or("packages missing")? {
        if !members.contains(&package["id"])
            || ["a4n-model-test-tools", "a4n-test-support"]
                .iter()
                .any(|name| package["name"] == *name)
        {
            continue;
        }
        dependencies.insert(
            package["name"]
                .as_str()
                .ok_or("package name missing")?
                .to_string(),
            package["dependencies"]
                .as_array()
                .ok_or("dependencies missing")?
                .iter()
                .filter_map(|dep| dep["name"].as_str())
                .filter(|name| name.starts_with("a4n-"))
                .collect::<Vec<_>>(),
        );
        let directory = Path::new(
            package["manifest_path"]
                .as_str()
                .ok_or("manifest path missing")?,
        )
        .parent()
        .ok_or("manifest parent missing")?
        .join("src");
        if !directory.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        source_files(&directory, &mut files)?;
        files.sort();
        for file in files {
            let syntax = syn::parse_file(&fs::read_to_string(&file)?)
                .map_err(|error| format!("{}: {error}", file.display()))?;
            let mut calls = Calls::default();
            calls.visit_file(&syntax);
            for name in calls.implementations {
                implementations.push(json!({"crate":package["name"],"file":file.strip_prefix(&root)?.to_str(),"trait":name}));
            }
            for mut row in calls.rows {
                row["crate"] = package["name"].clone();
                row["file"] = json!(
                    file.strip_prefix(&root)?
                        .to_str()
                        .ok_or("non UTF8 source path")?
                );
                rows.push(row);
            }
        }
    }
    let mut rows = discover(rows);
    rows.sort_by_key(Value::to_string);
    implementations.sort_by_key(Value::to_string);
    let catalogs = json!({
        "text":a4n_domain::llm::LLM_PROVIDER_CAPABILITIES.iter().map(|cap|json!({"id":cap.id,"protocol":format!("{:?}",cap.protocol)})).collect::<Vec<_>>(),
        "image":a4n_domain::image::IMAGE_PROVIDER_CAPABILITIES.iter().map(|cap|json!({"id":cap.backend,"references":cap.supports_reference_images})).collect::<Vec<_>>(),
        "video":a4n_video_provider_contract::VIDEO_PROVIDER_CAPABILITIES.iter().map(|cap|json!({"id":cap.backend,"asynchronous":cap.asynchronous,"last_frame":cap.last_frame,"max_reference_images":cap.max_reference_images,"reference_video_audio":cap.reference_video_audio,"first_frame_with_references":cap.first_frame_with_references,"status_query":cap.status_query,"delete":cap.delete,"fps_parameter":cap.fps_parameter})).collect::<Vec<_>>()
    });
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"schema_version":2,"scanner":"syn-workspace","dependencies":dependencies,"implementations":implementations,"calls":rows,"catalogs":catalogs})
        )?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("模型调用 AST 扫描失败：{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_finds_nested_and_qualified_calls_without_comments_strings_or_test_code() {
        let file = syn::parse_file(
            r#"
            fn chat_stream() {}
            fn run() { // client.chat_stream(fake);
                let _ = "client.embed(fake)";
                client.chat_stream(request);
                driver::execute_llm_generation(request);
                callback(|| embedding.embed(texts));
            }
            #[cfg(test)] mod tests { fn fake() { client.chat_stream(fake); } }
            #[test] fn check() { video.generate(fake); }
            #[tokio::test] async fn async_check() { video.generate(fake).await; }
            #[cfg(all(test, feature="http"))] mod isolated { fn fake() { video.generate(fake); } }
            #[cfg(any(test, feature="http"))] fn production() { video.generate(request); }
        "#,
        )
        .unwrap();
        let mut calls = Calls::default();
        calls.visit_file(&file);
        let rows = discover(calls.rows);
        assert_eq!(rows.len(), 4);
        assert_eq!(
            rows.iter().filter(|row| row["function"] == "run").count(),
            3
        );
        assert!(rows.iter().any(|row| row["function"] == "production"));
    }
}
