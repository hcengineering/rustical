use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use rustical_store::{auth::{AuthenticationProvider, User}, Error};
use super::HulyAuthProvider;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountsResponce {
    result: Option<AccountResult>,
    error: Option<AccountError>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountResult {
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountError {
    code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountRequest {
    method: String,
    params: Vec<String>,
}

async fn account_post(url: &str, req: &AccountRequest, token: Option<&str>) -> Result<AccountResult, Error> {
    let client = reqwest::Client::new();
    let mut response = client.post(url)
        .json(req);
    if let Some(token) = token {
        response = response.header("Authorization", format!("Bearer {}", token));
    }
    let response = response
        .send()
        .await;
    if let Err(err) = response {
        return Err(Error::ApiError(format!("{:?}", err)));
    }
    let response = response.unwrap();
    let body = response.json::<AccountsResponce>().await;
    if let Err(err) = body {
        return Err(Error::ApiError(format!("{:?}", err)));
    }
    let body = body.unwrap();
    if let Some(error) = body.error {
        return Err(Error::ApiError(format!("{:?}", error.code)));
    }
    if let Some(result) = body.result {
        return Ok(result);
    }
    Err(Error::ApiError("empty result".into()))
}

async fn login(url: &str, user: &str, password: &str) -> Result<String, Error> {
    let req = AccountRequest {
        method: "login".to_string(),
        params: vec![user.to_string(), password.to_string()],
    };
    let res = account_post(url, &req, None).await?;
    Ok(res.token)
}

async fn select_workspace(url: &str, token: &str, workspace: &str) -> Result<String, Error> {
    let req = AccountRequest {
        method: "selectWorkspace".to_string(),
        params: vec![workspace.to_string(), "external".to_string()],
    };
    let res = account_post(url, &req, Some(token)).await?;
    Ok(res.token)
}

#[async_trait]
impl AuthenticationProvider for HulyAuthProvider {
    async fn validate_user_token(&self, user_id: &str, password: &str) -> Result<Option<User>, Error> {
        let mut p = user_id.split("|");
        let user_id = p.next().unwrap();
        let workspace = p.next();

        tracing::debug!("Huly login user={} ws={}", user_id, workspace.unwrap_or_default());

        let token = login(&self.accounts_url, user_id, password).await;
        if let Err(err) = &token {
            tracing::error!("Error logging in: {:?}", err);
            // AuthenticationMiddleware can't handle errors, it crashes the request thread
            // Returning None will cause it to responce with 401 Unauthorized
            return Ok(None);
        }
        let mut token = token.unwrap();

        if let Some(workspace) = workspace {
            let ws_token = select_workspace(&self.accounts_url, &token, workspace).await;
            if let Err(err) = &ws_token {
                tracing::error!("Error selecting workspace: {:?}", err);
                // AuthenticationMiddleware can't handle errors, it crashes the request thread
                // Returning None will cause it to responce with 401 Unauthorized
                return Ok(None);
            }
            token = ws_token.unwrap();
        }

        Ok(Some(User {
            id: user_id.to_string(),
            displayname: Some(user_id.to_string()),
            password: Some(token),
            workspace: workspace.map(|w| w.to_string()),
        }))
    }
}
