fn get_request(path: &str) -> JsonRequest {
    JsonRequest {
        method: Method::GET,
        path: path.to_string(),
        body: None,
        authenticated: true,
    }
}

fn task_path(task_id: &str, attempt: u32, suffix: &str) -> Result<String> {
    validate_non_empty(task_id, "task_id")?;
    validate_attempt(attempt)?;
    Ok(format!(
        "/generation-tasks/{}{suffix}?attempt={attempt}",
        path_segment(task_id)
    ))
}

fn validate_attempt(attempt: u32) -> Result<()> {
    if attempt == 0 {
        bail!("attempt 必须为正整数");
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} 不能为空");
    }
    Ok(())
}

fn validate_optional_positive(value: Option<i64>, field: &str) -> Result<()> {
    if value.is_some_and(|value| value <= 0) {
        bail!("{field} 必须为正整数");
    }
    Ok(())
}

fn query_path<const N: usize>(base: &str, pairs: [(&str, Option<String>); N]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        if let Some(value) = value {
            serializer.append_pair(key, &value);
        }
    }
    let query = serializer.finish();
    if query.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{query}")
    }
}

fn path_segment(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn read_source(source: &str, label: &str) -> Result<String> {
    if source == "-" {
        let mut contents = String::new();
        std::io::stdin()
            .read_to_string(&mut contents)
            .with_context(|| format!("从 stdin 读取{label}失败"))?;
        return Ok(contents);
    }
    if let Some(path) = source.strip_prefix('@') {
        if path.is_empty() {
            bail!("@ 后必须提供{label}文件路径");
        }
        return std::fs::read_to_string(path)
            .with_context(|| format!("读取{label}文件失败: {path}"));
    }
    Ok(source.to_string())
}

pub(crate) fn read_json_source(source: &str) -> Result<Value> {
    serde_json::from_str(&read_source(source, "JSON ")?).context("输入不是有效 JSON")
}

fn read_text_source(source: &str) -> Result<String> {
    read_source(source, "文本 ")
}

struct FileAssignment {
    pointer: String,
    path: PathBuf,
}

fn apply_file_inputs(
    args: &mut Value,
    base64_files: &[String],
    text_files: &[String],
) -> Result<()> {
    for (raw, encoding) in base64_files
        .iter()
        .map(|raw| (raw, FileEncoding::Base64))
        .chain(text_files.iter().map(|raw| (raw, FileEncoding::Text)))
    {
        let assignment = parse_file_assignment(raw, false)?;
        let value = match encoding {
            FileEncoding::Base64 => {
                let bytes = std::fs::read(&assignment.path)
                    .with_context(|| format!("读取文件失败: {}", assignment.path.display()))?;
                Value::String(base64::engine::general_purpose::STANDARD.encode(bytes))
            }
            FileEncoding::Text => {
                Value::String(std::fs::read_to_string(&assignment.path).with_context(|| {
                    format!("读取 UTF-8 文件失败: {}", assignment.path.display())
                })?)
            }
        };
        set_json_pointer(args, &assignment.pointer, value)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum FileEncoding {
    Base64,
    Text,
}

struct OutputAssignment {
    file: FileAssignment,
    encoding: FileEncoding,
}

fn prepare_outputs(
    base64_outputs: &[String],
    text_outputs: &[String],
    force: bool,
) -> Result<Vec<OutputAssignment>> {
    if force && base64_outputs.is_empty() && text_outputs.is_empty() {
        bail!("--force 只能与 --save-base64 或 --save-text 一起使用");
    }
    let assignments = base64_outputs
        .iter()
        .map(|raw| (raw, FileEncoding::Base64))
        .chain(text_outputs.iter().map(|raw| (raw, FileEncoding::Text)))
        .map(|(raw, encoding)| {
            parse_file_assignment(raw, true).map(|file| OutputAssignment { file, encoding })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut paths = HashSet::new();
    for assignment in &assignments {
        if !paths.insert(assignment.file.path.clone()) {
            bail!("重复的输出文件: {}", assignment.file.path.display());
        }
        if assignment.file.path.exists() && !force {
            bail!(
                "输出文件已存在；使用 --force 覆盖: {}",
                assignment.file.path.display()
            );
        }
    }
    Ok(assignments)
}

fn save_outputs(value: &mut Value, assignments: &[OutputAssignment], force: bool) -> Result<()> {
    let decoded = assignments
        .iter()
        .map(|assignment| {
            let source = value
                .pointer(&assignment.file.pointer)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "响应 {} 不是字符串",
                        display_pointer(&assignment.file.pointer)
                    )
                })?;
            let bytes = match assignment.encoding {
                FileEncoding::Base64 => base64::engine::general_purpose::STANDARD
                    .decode(source)
                    .with_context(|| {
                        format!(
                            "响应 {} 不是有效 Base64",
                            display_pointer(&assignment.file.pointer)
                        )
                    })?,
                FileEncoding::Text => source.as_bytes().to_vec(),
            };
            Ok((assignment, bytes))
        })
        .collect::<Result<Vec<_>>>()?;

    for (assignment, bytes) in decoded {
        write_output_file(&assignment.file.path, &bytes, force)?;
        let replacement = json!({
            "saved_to": assignment.file.path,
            "byte_size": bytes.len()
        });
        replace_response_pointer(value, &assignment.file.pointer, replacement)?;
    }
    Ok(())
}

fn replace_response_pointer(value: &mut Value, pointer: &str, replacement: Value) -> Result<()> {
    let target = value
        .pointer_mut(pointer)
        .ok_or_else(|| anyhow::anyhow!("响应 JSON Pointer 不存在: {}", display_pointer(pointer)))?;
    *target = replacement;
    Ok(())
}

fn display_pointer(pointer: &str) -> &str {
    if pointer.is_empty() {
        "<root>"
    } else {
        pointer
    }
}

fn parse_file_assignment(raw: &str, allow_root: bool) -> Result<FileAssignment> {
    let (pointer, path) = raw
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("文件参数必须使用 POINTER=PATH: {raw}"))?;
    if (!allow_root || !pointer.is_empty()) && !pointer.starts_with('/') {
        bail!("JSON Pointer 必须以 / 开头: {pointer}");
    }
    if path.is_empty() {
        bail!("文件路径不能为空: {raw}");
    }
    Ok(FileAssignment {
        pointer: pointer.to_string(),
        path: PathBuf::from(path),
    })
}

fn set_json_pointer(root: &mut Value, pointer: &str, value: Value) -> Result<()> {
    let tokens = pointer
        .strip_prefix('/')
        .ok_or_else(|| anyhow::anyhow!("JSON Pointer 必须以 / 开头: {pointer}"))?
        .split('/')
        .map(decode_pointer_token)
        .collect::<Result<Vec<_>>>()?;
    let mut current = root;
    for (index, token) in tokens.iter().enumerate() {
        let last = index + 1 == tokens.len();
        match current {
            Value::Object(object) => {
                if last {
                    object.insert(token.clone(), value);
                    return Ok(());
                }
                current = object.entry(token).or_insert_with(|| json!({}));
            }
            Value::Array(array) => {
                let index = token
                    .parse::<usize>()
                    .map_err(|_| anyhow::anyhow!("数组 JSON Pointer 必须使用索引: {pointer}"))?;
                current = array
                    .get_mut(index)
                    .ok_or_else(|| anyhow::anyhow!("数组 JSON Pointer 越界: {pointer}"))?;
                if last {
                    *current = value;
                    return Ok(());
                }
            }
            Value::Null => {
                *current = json!({});
                let Value::Object(object) = current else {
                    unreachable!();
                };
                if last {
                    object.insert(token.clone(), value);
                    return Ok(());
                }
                current = object.entry(token).or_insert_with(|| json!({}));
            }
            _ => bail!("JSON Pointer 的父节点不是 object 或 array: {pointer}"),
        }
    }
    bail!("JSON Pointer 不能为空")
}

fn decode_pointer_token(token: &str) -> Result<String> {
    let mut decoded = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(ch) = chars.next() {
        if ch != '~' {
            decoded.push(ch);
            continue;
        }
        match chars.next() {
            Some('0') => decoded.push('~'),
            Some('1') => decoded.push('/'),
            _ => bail!("JSON Pointer 包含无效 ~ 转义: {token}"),
        }
    }
    Ok(decoded)
}

fn read_password(password_stdin: bool) -> Result<String> {
    read_secret_line(
        password_stdin,
        "必须使用 --password-stdin 从 stdin 提供密码，避免密码进入 shell history",
    )
}

fn read_password_pair(passwords_stdin: bool) -> Result<(String, String)> {
    if !passwords_stdin {
        bail!("必须使用 --passwords-stdin；stdin 第一行旧密码、第二行新密码");
    }
    let mut old_password = String::new();
    let mut new_password = String::new();
    std::io::stdin()
        .read_line(&mut old_password)
        .context("从 stdin 读取旧密码失败")?;
    std::io::stdin()
        .read_line(&mut new_password)
        .context("从 stdin 读取新密码失败")?;
    let old_password = trim_secret_line(old_password)?;
    let new_password = trim_secret_line(new_password)?;
    Ok((old_password, new_password))
}

fn read_secret_line(enabled: bool, missing_message: &str) -> Result<String> {
    if !enabled {
        bail!(missing_message.to_string());
    }
    let mut value = String::new();
    std::io::stdin()
        .read_line(&mut value)
        .context("从 stdin 读取敏感信息失败")?;
    trim_secret_line(value)
}

fn trim_secret_line(value: String) -> Result<String> {
    let value = value.trim_end_matches(['\r', '\n']).to_string();
    if value.is_empty() {
        bail!("敏感信息不能为空");
    }
    Ok(value)
}

fn output_body(body: ApiBody, output: &OutputOptions) -> Result<()> {
    if body
        .content_type
        .as_deref()
        .is_some_and(|value| value.contains("json") || value.starts_with("text/"))
    {
        return print_json(&body.into_value()?, output);
    }
    std::io::stdout()
        .write_all(&body.bytes)
        .context("写入 stdout 失败")
}

async fn write_response_file(path: &Path, mut response: Response, force: bool) -> Result<u64> {
    let expected_length = response.content_length();
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("创建临时输出文件失败: {}", parent.display()))?;
    let (file, temp_path) = temp.into_parts();
    let mut file = tokio::fs::File::from_std(file);

    let result = async {
        let mut written = 0_u64;
        while let Some(chunk) = response.chunk().await.map_err(client::transport_error)? {
            file.write_all(&chunk)
                .await
                .with_context(|| format!("写入输出文件失败: {}", path.display()))?;
            written = written
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| anyhow::anyhow!("下载文件大小溢出 u64"))?;
        }
        if let Some(expected) = expected_length
            && expected != written
        {
            bail!("下载响应长度不匹配: expected={} actual={written}", expected);
        }
        file.sync_all()
            .await
            .with_context(|| format!("同步输出文件失败: {}", path.display()))?;
        Ok(written)
    }
    .await;

    drop(file);
    let written = result?;
    if force {
        temp_path.persist(path)
    } else {
        temp_path.persist_noclobber(path)
    }
    .map_err(|error| error.error)
    .with_context(|| {
        if path.exists() && !force {
            format!("输出文件已存在；使用 --force 覆盖: {}", path.display())
        } else {
            format!("原子写入输出文件失败: {}", path.display())
        }
    })?;
    Ok(written)
}

fn ensure_output_available(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        bail!("输出文件已存在；使用 --force 覆盖: {}", path.display());
    }
    Ok(())
}

fn write_output_file(path: &Path, bytes: &[u8], force: bool) -> Result<()> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("创建临时输出文件失败: {}", parent.display()))?;
    temp.write_all(bytes)
        .with_context(|| format!("写入输出文件失败: {}", path.display()))?;
    temp.as_file_mut()
        .sync_all()
        .with_context(|| format!("同步输出文件失败: {}", path.display()))?;
    if force {
        temp.persist(path)
    } else {
        temp.persist_noclobber(path)
    }
    .map_err(|error| error.error)
    .with_context(|| {
        if path.exists() && !force {
            format!("输出文件已存在；使用 --force 覆盖: {}", path.display())
        } else {
            format!("原子写入输出文件失败: {}", path.display())
        }
    })?;
    Ok(())
}

fn print_json(value: &Value, output: &OutputOptions) -> Result<()> {
    output::print_value(value, output)
}

