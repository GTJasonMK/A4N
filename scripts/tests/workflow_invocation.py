def auth_invocation(
    command: str,
    payload: dict[str, Any],
    allow_destructive: bool,
) -> tuple[list[str], str | None]:
    input_value = payload.get("input", {})
    if not isinstance(input_value, dict):
        input_value = {}
    if command == "login_user":
        return ["auth", "login", "--username", str(input_value.get("username", "")), "--password-stdin"], f"{input_value.get('password', '')}\n"
    if command == "register_user":
        return [
            "auth", "register", "--username", str(input_value.get("username", "")),
            "--email", str(input_value.get("email", "")),
            "--invite-code", str(input_value.get("invite_code", "")),
            "--password-stdin",
        ], f"{input_value.get('password', '')}\n"
    if command == "refresh_token":
        return ["auth", "refresh"], None
    if command == "change_user_password":
        return ["auth", "change-password", "--passwords-stdin"], f"{input_value.get('old_password', '')}\n{input_value.get('new_password', '')}\n"
    if command == "oauth2_callback":
        return ["auth", "oauth-callback", str(input_value.get("provider", "")), "--code-stdin"], f"{input_value.get('code', '')}\n"
    if command == "revoke_session":
        parts = ["auth", "revoke-session", str(input_value.get("session_id", ""))]
        if allow_destructive:
            parts.append("--yes")
        return parts, None
    raise RunnerError(f"不支持的 auth command: {command}")


AUTH_CLI_REQUIRED_FIELDS: dict[str, tuple[str, ...]] = {
    "login_user": ("server", "user", "session"),
    "register_user": ("server", "user", "session"),
    "refresh_token": ("refreshed", "session_id", "token_type", "expires_in"),
    "change_user_password": ("changed",),
    "oauth2_callback": ("server", "user", "session"),
    "revoke_session": ("revoked",),
}


def validate_auth_cli_result(command: str, value: Any) -> None:
    required = AUTH_CLI_REQUIRED_FIELDS.get(command)
    if required is None:
        return
    if not isinstance(value, dict):
        raise ValueError("/data: auth CLI 输出应为 object")
    for field in required:
        if field not in value:
            raise ValueError(f"/data/{field}: CLI auth 输出缺少字段")


def invoke(
    cli: Cli,
    command: str,
    payload: dict[str, Any],
    allow_destructive: bool,
    descriptor: dict[str, Any] | None = None,
    extra_parts: list[str] | None = None,
) -> dict[str, Any]:
    temp_config: Path | None = None
    invoke_cli = cli
    stdin: str | None = None
    if command in AUTH_COMMANDS:
        fd, temp_name = tempfile.mkstemp(prefix="a4n-auth-probe-", suffix=".json")
        os.close(fd)
        temp_config = Path(temp_name)
        shutil.copy2(cli.config, temp_config)
        invoke_cli = Cli(cli.executable, cli.server, temp_config, cli.context, cli.timeout, cli.env)
        parts, stdin = auth_invocation(command, payload, allow_destructive)
    else:
        parts = ["call", command, "--args", json.dumps(payload, ensure_ascii=False)]
        parts += extra_parts or []
        if allow_destructive:
            parts.append("--allow-destructive")
    try:
        raw = invoke_cli.run(parts, stdin)
    finally:
        if temp_config:
            temp_config.unlink(missing_ok=True)
            temp_config.with_name(temp_config.name + ".lock").unlink(missing_ok=True)
    envelope = raw["stdout_documents"][-1] if raw["stdout_documents"] else None
    ok = raw["exit_code"] == 0 and isinstance(envelope, dict) and envelope.get("ok") is True
    result: dict[str, Any] = {
        "exit_code": raw["exit_code"],
        "duration_ms": raw["duration_ms"],
        "ok": ok,
        "data": envelope.get("data") if isinstance(envelope, dict) else None,
        "meta": envelope.get("meta") if isinstance(envelope, dict) else None,
    }
    if not ok:
        result["error"] = command_error(raw)
    elif command in AUTH_CLI_REQUIRED_FIELDS:
        try:
            validate_auth_cli_result(command, result["data"])
        except ValueError as error:
            result["ok"] = False
            result["error"] = {
                "code": "invalid_cli_result",
                "kind": "client",
                "message": str(error),
            }
    elif descriptor:
        try:
            validate_result_schema(descriptor.get("result_schema"), result["data"], descriptor.get("result_schema", {}))
            validate_result_references(descriptor, envelope)
        except ValueError as error:
            result["ok"] = False
            result["error"] = {
                "code": "invalid_result_schema",
                "kind": "client",
                "message": str(error),
            }
    result["classification"] = classify(result, descriptor)
    return result


def classify(result: dict[str, Any], descriptor: dict[str, Any] | None = None) -> str:
    if result.get("ok") is True:
        if descriptor and descriptor.get("effect") == "destructive":
            return "destructive_success"
        return "business_success"
    error = result.get("error") or {}
    code = error.get("code") if isinstance(error, dict) else None
    status = error.get("http_status") if isinstance(error, dict) else None
    message = error.get("message", "") if isinstance(error, dict) else str(error)
    if result.get("exit_code") == 124 or code in {None, "invalid_cli_output"}:
        return "real_failure"
    if code == "invalid_result_schema":
        return "real_failure"
    if any(marker in message for marker in ("数据库错误", "No deterministic fixture matched", "LLM API 返回错误", "provider_job_failed", "内部错误")):
        return "real_failure"
    if status in {401, 403} or code in {"forbidden", "unauthorized", "permission_denied"}:
        return "permission_rejection"
    if isinstance(status, int) and 400 <= status < 500:
        return "expected_state_rejection"
    if code in {
        "destructive_confirmation_required",
        "invalid_input",
        "invalid_request",
        "invalid_argument",
        "not_found",
        "invalid_state",
        "conflict",
        "forbidden",
        "unauthorized",
        "unsupported",
        "capability_unavailable",
    } or result.get("exit_code") in {2, 3, 4, 5, 6, 10}:
        return "expected_state_rejection"
    return "real_failure"


def fixture_result_classification(result: dict[str, Any]) -> str:
    """Fixture setup errors fail the run; only successful setup calls pass."""
    return result["classification"] if result.get("ok") is True else "real_failure"

