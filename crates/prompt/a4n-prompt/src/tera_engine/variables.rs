use std::collections::HashSet;

use tera::ast::{Expr, ExprVal, FunctionCall, MacroDefinition, Node};

pub(super) fn missing_template_variables(
    content: &str,
    variables: &HashSet<String>,
) -> Result<Vec<String>, tera::Error> {
    let template = tera::Template::new("__prompt_variable_validation__", None, content)?;
    let mut collector = VariableCollector::default();
    let mut scopes = vec![HashSet::new()];
    collector.collect_nodes(&template.ast, &mut scopes);

    let provided: HashSet<&str> = variables.iter().map(|s| s.as_str()).collect();
    Ok(collector
        .required
        .into_iter()
        .filter(|v| !provided.contains(v.as_str()))
        .collect())
}

pub fn required_template_variables(content: &str) -> Result<Vec<String>, tera::Error> {
    let template = tera::Template::new("__prompt_variable_validation__", None, content)?;
    let mut collector = VariableCollector::default();
    collector.collect_nodes(&template.ast, &mut vec![HashSet::new()]);
    collector.required.sort();
    Ok(collector.required)
}

pub fn template_variable_access_paths(content: &str) -> Result<Vec<Vec<String>>, tera::Error> {
    let template = tera::Template::new("__prompt_variable_paths__", None, content)?;
    let mut collector = VariableCollector::default();
    collector.collect_nodes(&template.ast, &mut vec![HashSet::new()]);
    collector.access_paths.sort();
    collector.access_paths.dedup();
    Ok(collector.access_paths)
}

#[derive(Default)]
struct VariableCollector {
    required: Vec<String>,
    access_paths: Vec<Vec<String>>,
}

impl VariableCollector {
    fn collect_nodes(&mut self, nodes: &[Node], scopes: &mut Vec<HashSet<String>>) {
        for node in nodes {
            match node {
                Node::VariableBlock(_, expr) => self.collect_expr(expr, scopes),
                Node::Set(_, set) => {
                    self.collect_expr(&set.value, scopes);
                    define_local(scopes, &set.key, set.global);
                }
                Node::Forloop(_, for_loop, _) => {
                    self.collect_expr(&for_loop.container, scopes);

                    scopes.push(HashSet::new());
                    if let Some(key) = &for_loop.key {
                        define_local(scopes, key, false);
                    }
                    define_local(scopes, &for_loop.value, false);
                    define_local(scopes, "loop", false);
                    self.collect_nodes(&for_loop.body, scopes);
                    scopes.pop();

                    if let Some(empty_body) = &for_loop.empty_body {
                        scopes.push(HashSet::new());
                        self.collect_nodes(empty_body, scopes);
                        scopes.pop();
                    }
                }
                Node::If(if_node, _) => {
                    for (_, condition, body) in &if_node.conditions {
                        self.collect_expr(condition, scopes);
                        scopes.push(HashSet::new());
                        self.collect_nodes(body, scopes);
                        scopes.pop();
                    }
                    if let Some((_, body)) = &if_node.otherwise {
                        scopes.push(HashSet::new());
                        self.collect_nodes(body, scopes);
                        scopes.pop();
                    }
                }
                Node::FilterSection(_, filter_section, _) => {
                    self.collect_function_call(&filter_section.filter, scopes);
                    self.collect_nodes(&filter_section.body, scopes);
                }
                Node::Block(_, block, _) => {
                    scopes.push(HashSet::new());
                    self.collect_nodes(&block.body, scopes);
                    scopes.pop();
                }
                Node::MacroDefinition(_, macro_def, _) => self.collect_macro(macro_def, scopes),
                Node::Super
                | Node::Text(_)
                | Node::Extends(_, _)
                | Node::Include(_, _, _)
                | Node::ImportMacro(_, _, _)
                | Node::Raw(_, _, _)
                | Node::Break(_)
                | Node::Continue(_)
                | Node::Comment(_, _) => {}
            }
        }
    }

    fn collect_macro(&mut self, macro_def: &MacroDefinition, scopes: &mut Vec<HashSet<String>>) {
        for default_value in macro_def.args.values().flatten() {
            self.collect_expr(default_value, scopes);
        }

        scopes.push(HashSet::new());
        for arg in macro_def.args.keys() {
            define_local(scopes, arg, false);
        }
        self.collect_nodes(&macro_def.body, scopes);
        scopes.pop();
    }

    fn collect_expr(&mut self, expr: &Expr, scopes: &[HashSet<String>]) {
        self.collect_expr_val(&expr.val, scopes);
        for filter in &expr.filters {
            self.collect_function_call(filter, scopes);
        }
    }

    fn collect_expr_val(&mut self, value: &ExprVal, scopes: &[HashSet<String>]) {
        match value {
            ExprVal::Ident(ident) => self.require_ident(ident, scopes),
            ExprVal::Math(expr) => {
                self.collect_expr(&expr.lhs, scopes);
                self.collect_expr(&expr.rhs, scopes);
            }
            ExprVal::Logic(expr) => {
                self.collect_expr(&expr.lhs, scopes);
                self.collect_expr(&expr.rhs, scopes);
            }
            ExprVal::Test(test) => {
                self.require_ident(&test.ident, scopes);
                for arg in &test.args {
                    self.collect_expr(arg, scopes);
                }
            }
            ExprVal::MacroCall(call) => {
                for arg in call.args.values() {
                    self.collect_expr(arg, scopes);
                }
            }
            ExprVal::FunctionCall(call) => self.collect_function_call(call, scopes),
            ExprVal::Array(items) => {
                for item in items {
                    self.collect_expr(item, scopes);
                }
            }
            ExprVal::StringConcat(concat) => {
                for value in &concat.values {
                    self.collect_expr_val(value, scopes);
                }
            }
            ExprVal::In(expr) => {
                self.collect_expr(&expr.lhs, scopes);
                self.collect_expr(&expr.rhs, scopes);
            }
            ExprVal::String(_) | ExprVal::Int(_) | ExprVal::Float(_) | ExprVal::Bool(_) => {}
        }
    }

    fn collect_function_call(&mut self, call: &FunctionCall, scopes: &[HashSet<String>]) {
        for arg in call.args.values() {
            self.collect_expr(arg, scopes);
        }
    }

    fn require_ident(&mut self, ident: &str, scopes: &[HashSet<String>]) {
        let Some(root) = root_ident(ident) else {
            return;
        };
        if scopes.iter().rev().any(|scope| scope.contains(root)) {
            return;
        }
        if !self.required.iter().any(|existing| existing == root) {
            self.required.push(root.to_string());
        }
        if let Some(path) = variable_access_path(ident) {
            self.access_paths.push(path);
        }
    }
}

fn define_local(scopes: &mut [HashSet<String>], ident: &str, global: bool) {
    let Some(root) = root_ident(ident) else {
        return;
    };
    let target = if global {
        scopes.first_mut()
    } else {
        scopes.last_mut()
    };
    if let Some(scope) = target {
        scope.insert(root.to_string());
    }
}

fn root_ident(ident: &str) -> Option<&str> {
    let ident = ident.trim();
    if ident.is_empty() {
        return None;
    }

    let end = ident.find(['.', '[']).unwrap_or(ident.len());
    let root = &ident[..end];
    if is_identifier(root) {
        Some(root)
    } else {
        None
    }
}

fn variable_access_path(ident: &str) -> Option<Vec<String>> {
    let path = ident
        .split('.')
        .map(str::trim)
        .map(str::to_string)
        .collect::<Vec<_>>();
    (path.len() > 1 && path.iter().all(|part| is_identifier(part))).then_some(path)
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}
