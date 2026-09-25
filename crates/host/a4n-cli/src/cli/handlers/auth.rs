async fn run_auth(
    client: &mut ApiClient,
    command: AuthCommand,
    output: &OutputOptions,
) -> Result<()> {
    match command {
        AuthCommand::Login {
            username,
            password_stdin,
        } => {
            let password = read_password(password_stdin)?;
            let auth = client.login(username, password).await?;
            print_json(
                &json!({
                    "server": client.api_base(),
                    "user": auth.user,
                    "session": auth.session
                }),
                output,
            )
        }
        AuthCommand::Register {
            username,
            email,
            invite_code,
            password_stdin,
        } => {
            let password = read_password(password_stdin)?;
            let auth = client
                .register(username, email, invite_code, password)
                .await?;
            print_json(
                &json!({
                    "server": client.api_base(),
                    "user": auth.user,
                    "session": auth.session
                }),
                output,
            )
        }
        AuthCommand::Status => {
            let user = client.current_user().await?;
            print_json(&serde_json::to_value(user)?, output)
        }
        AuthCommand::Refresh => {
            let tokens = client.refresh().await?;
            print_json(
                &json!({
                    "refreshed": true,
                    "session_id": tokens.session_id,
                    "token_type": tokens.token_type,
                    "expires_in": tokens.expires_in
                }),
                output,
            )
        }
        AuthCommand::ChangePassword { passwords_stdin } => {
            let (old_password, new_password) = read_password_pair(passwords_stdin)?;
            let secrets = [old_password.clone(), new_password.clone()];
            client
                .send_json(routing::command_request(
                    "change_user_password",
                    json!({
                        "input": ChangePasswordInput { old_password, new_password }
                    }),
                )?)
                .await
                .map_err(|error| client::redact_error_values(error, &secrets))?;
            print_json(&json!({ "password_changed": true }), output)
        }
        AuthCommand::UpdateProfile {
            email,
            avatar_url,
            clear_avatar_url,
        } => {
            if email.is_none() && avatar_url.is_none() && !clear_avatar_url {
                bail!("至少提供 --email、--avatar-url 或 --clear-avatar-url 之一");
            }
            let mut input = Map::new();
            if let Some(email) = email {
                input.insert("email".to_string(), Value::String(email));
            }
            if clear_avatar_url {
                input.insert("avatar_url".to_string(), Value::Null);
            } else if let Some(avatar_url) = avatar_url {
                input.insert("avatar_url".to_string(), Value::String(avatar_url));
            }
            let value = client
                .send_json(routing::command_request(
                    "update_user_profile",
                    json!({ "input": Value::Object(input) }),
                )?)
                .await?;
            print_json(&value, output)
        }
        AuthCommand::Sessions => {
            let value = client
                .send_json(routing::command_request("list_sessions", json!({}))?)
                .await?;
            print_json(&value, output)
        }
        AuthCommand::RevokeSession { session_id, yes } => {
            if !yes {
                return Err(output::CliError::new(
                    "destructive_confirmation_required",
                    "confirmation",
                    "auth revoke-session 需要 --yes 显式授权",
                    10,
                )
                .into());
            }
            validate_non_empty(&session_id, "session_id")?;
            let current_session = client.session_id().map(str::to_string);
            client
                .send_json(routing::command_request(
                    "revoke_session",
                    json!({
                        "input": RevokeSessionInput {
                            session_id: session_id.clone()
                        }
                    }),
                )?)
                .await?;
            let revoked_current = current_session.as_deref() == Some(session_id.as_str());
            if revoked_current {
                client.clear_auth()?;
            }
            print_json(
                &json!({ "revoked": true, "session_id": session_id, "logged_out": revoked_current }),
                output,
            )
        }
        AuthCommand::OauthAuthorize {
            provider,
            redirect_uri,
        } => {
            let url = client.oauth2_authorize(provider, redirect_uri).await?;
            print_json(&json!(url), output)
        }
        AuthCommand::OauthCallback {
            provider,
            code_stdin,
        } => {
            let code = read_secret_line(
                code_stdin,
                "必须使用 --code-stdin 从 stdin 提供 OAuth2 完整回调 URL",
            )?;
            let auth = client.oauth2_callback(provider, code).await?;
            print_json(
                &json!({
                    "server": client.api_base(),
                    "user": auth.user,
                    "session": auth.session
                }),
                output,
            )
        }
        AuthCommand::Logout => {
            client.logout().await?;
            print_json(&json!({ "logged_out": true }), output)
        }
    }
}
