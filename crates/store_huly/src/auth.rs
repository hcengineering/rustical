use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use rustical_store::{auth::{AuthenticationProvider, User}, Error};
use crate::api::{get_account, ApiAuth};

use super::HulyAuthProvider;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountsResponce<TResult>
{
    result: Option<TResult>,
    error: Option<AccountError>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountLoginResult {
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountWorkspaceResult {
    workspace: String,
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

async fn account_post<TResult>(url: &str, req: &AccountRequest, token: Option<&str>) -> Result<TResult, Error>
where
    TResult: for<'de> Deserialize<'de>
{
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
    let body = response.json::<AccountsResponce<TResult>>().await;
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
    let res = account_post::<AccountLoginResult>(url, &req, None).await?;
    Ok(res.token)
}

async fn select_workspace(url: &str, token: &str, workspace: &str) -> Result<String, Error> {
    let req = AccountRequest {
        method: "selectWorkspace".to_string(),
        params: vec![workspace.to_string(), "external".to_string()],
    };
    let res = account_post::<AccountLoginResult>(url, &req, Some(token)).await?;
    Ok(res.token)
}

pub(crate) async fn get_workspaces(url: &str, token: &str) -> Result<Vec<String>, Error> {
    let req = AccountRequest {
        method: "getUserWorkspaces".to_string(),
        params: vec![],
    };
    let res = account_post::<Vec<AccountWorkspaceResult>>(url, &req, Some(token)).await?;
    Ok(res.into_iter().map(|w| w.workspace).collect())
}

#[async_trait]
impl AuthenticationProvider for HulyAuthProvider {
    async fn validate_user_token(&self, user_id: &str, password: &str) -> Result<Option<User>, Error> {
        let mut p = user_id.split("|");
        let user_id = p.next().unwrap();
        let workspace = p.next();

        tracing::debug!("HULY_LOGIN user={} ws={:?}", user_id, workspace);

        let token = login(&self.accounts_url, user_id, password).await;
        if let Err(err) = &token {
            tracing::error!("Error logging in: {:?}", err);
            // AuthenticationMiddleware can't handle errors, it crashes the request thread
            // Returning None will cause it to responce with 401 Unauthorized
            return Ok(None);
        }
        let token = token.unwrap();

        let Some(workspace) = workspace else {
            return Ok(Some(User {
                id: user_id.to_string(),
                displayname: Some(user_id.to_string()),
                password: Some(token.clone()),
                workspace: None,
                account: None,
            }));
        };

        let ws_token = select_workspace(&self.accounts_url, &token, workspace).await;
        if let Err(err) = &ws_token {
            tracing::error!("Error selecting workspace: {:?}", err);
            // AuthenticationMiddleware can't handle errors, it crashes the request thread
            // Returning None will cause it to responce with 401 Unauthorized
            return Ok(None);
        }
        let ws_token = ws_token.unwrap();
        
        let account = get_account(&self.api_url, ApiAuth { 
            token: ws_token.clone(),
            workspace: workspace.to_string(),
        }).await;
        if let Err(err) = &account {
            tracing::error!("Error getting account: {:?}", err);
            // AuthenticationMiddleware can't handle errors, it crashes the request thread
            // Returning None will cause it to responce with 401 Unauthorized
            return Ok(None);
        }
        let account = account.unwrap();

        Ok(Some(User {
            id: user_id.to_string(),
            displayname: Some(user_id.to_string()),
            password: Some(ws_token),
            workspace: Some(workspace.to_string()),
            account: Some(account.id),
        }))
    }
}
