use std::collections::HashMap;
use async_trait::async_trait;
use rustical_store::{
    auth::{AuthenticationProvider, User},
    Error,
};
use serde::{Deserialize, Serialize};
use crate::api::{find_all, FindOptions, FindParams, HulyPerson, CLASS_PERSON};
use super::HulyAuthProvider;

#[derive(Debug, Clone)]
pub(crate) struct HulyUser {
    pub(crate) id: String,
    pub(crate) contact_id: String,
    pub(crate) social_id: String,
    pub(crate) account_uuid: String,
    pub(crate) workspace_url: String,
    pub(crate) workspace_uuid: String,
    pub(crate) token: String,
    pub(crate) api_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountsResponce<TResult> {
    result: Option<TResult>,
    error: Option<AccountError>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginInfo {
    account: String,
    token: String,
    social_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceLoginInfo {
    token: String,
    endpoint: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceInfo {
    uuid: String,
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountError {
    code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountRequest<'a> {
    method: &'static str,
    params: AccountRequestParams<'a>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct AccountRequestParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
}

async fn account_post<'a, TResult>(
    url: &str,
    req: &AccountRequest<'a>,
    token: Option<&str>,
) -> Result<TResult, Error>
where
    TResult: for<'de> Deserialize<'de>,
{
    let client = reqwest::Client::new();
    let mut response = client.post(url).json(req);
    if let Some(token) = token {
        response = response.header("Authorization", format!("Bearer {}", token));
    }
    let response = response.send().await;
    if let Err(err) = response {
        return Err(Error::ApiError(format!("Account servce: {:?}", err)));
    }
    let response = response.unwrap();
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await;
        if let Err(err) = text {
            //println!("*** ACCOUNT: resp_error: {:?}", err);
            return Err(Error::ApiError(format!(
                "Account servce: resp_error {}: {:?}",
                status, err
            )));
        }
        let text = text.unwrap();
        //println!("*** ACCUNT: status_error: {:?}", text);
        return Err(Error::ApiError(format!(
            "Account servce: status_error {}: {:?}",
            status, text
        )));
    }
    let text = response.text().await;
    if let Err(err) = text {
        //println!("*** ACCOUNT: resp_error: {:?}", err);
        return Err(Error::ApiError(format!(
            "Account servce: resp_error: {:?}",
            err
        )));
    }
    let text = text.unwrap();
    //println!("*** ACCOUNT: json_text: {:?}", text);
    let result = serde_json::from_str::<AccountsResponce<TResult>>(&text);
    if let Err(err) = &result {
        //println!("*** ACCOUNT: json_error: {:?}", err);
        return Err(Error::ApiError(format!(
            "Account servce: json_error: {:?}",
            err
        )));
    }
    let result = result.unwrap();
    if let Some(error) = result.error {
        return Err(Error::ApiError(format!("Account servce: {:?}", error.code)));
    }
    if let Some(result) = result.result {
        return Ok(result);
    }
    Err(Error::ApiError("Account servce: empty result".into()))
}

async fn login(url: &str, email: &str, password: &str) -> Result<LoginInfo, Error> {
    let req = AccountRequest {
        method: "login",
        params: AccountRequestParams {
            email: Some(email),
            password: Some(password),
            ..Default::default()
        },
    };
    let res = account_post::<LoginInfo>(url, &req, None).await?;
    Ok(res)
}

fn extract_api_url(endpoint: &str) -> String {
    endpoint
        .replace("ws://", "http://")
        .replace("wss://", "https://")
        .trim_matches('/')
        .to_string()
}

#[test]
fn test_extract_api_url() {
    assert_eq!(
        extract_api_url("ws://transactor.hc.engineering"),
        "http://transactor.hc.engineering"
    );
    assert_eq!(
        extract_api_url("wss://transactor.hc.engineering"),
        "https://transactor.hc.engineering"
    );
    assert_eq!(
        extract_api_url("ws://transactor.hc.engineering/"),
        "http://transactor.hc.engineering"
    );
    assert_eq!(
        extract_api_url("https://transactor.hc.engineering/"),
        "https://transactor.hc.engineering"
    );
}

async fn select_workspace(
    url: &str,
    token: &str,
    workspace: &str,
) -> Result<(String, String), Error> {
    let req = AccountRequest {
        method: "selectWorkspace",
        params: AccountRequestParams {
            workspace_url: Some(workspace),
            kind: Some("external"),
            ..Default::default()
        },
    };
    let res = account_post::<WorkspaceLoginInfo>(url, &req, Some(token)).await?;
    Ok((res.token, extract_api_url(&res.endpoint)))
}

pub(crate) async fn get_workspaces(url: &str, token: &str) -> Result<Vec<(String, String)>, Error> {
    let req = AccountRequest {
        method: "getUserWorkspaces",
        params: Default::default(),
    };
    let res = account_post::<Vec<WorkspaceInfo>>(url, &req, Some(token)).await?;
    Ok(res.into_iter().map(|w| (w.url, w.uuid)).collect())
}

fn make_user(user: &HulyUser) -> User {
    User {
        id: user.id.clone(),
        displayname: Some(user.id.clone()),
        password: None,
        app_tokens: vec![],
        memberships: vec![],
        principal_type: rustical_store::auth::user::PrincipalType::Individual,
    }
}

#[async_trait]
impl AuthenticationProvider for HulyAuthProvider {
    async fn validate_user_token(
        &self,
        id_and_ws: &str,
        password: &str,
    ) -> Result<Option<User>, Error> {
        let mut p = id_and_ws.split("|");
        let user_id = p.next().unwrap();
        let ws_url = p.next();

        tracing::debug!("HULY_LOGIN user_id={} ws_url={:?}", user_id, ws_url);

        let mut cache = self.calendar_cache.lock().await;
        if let Some(huly_user) = cache.try_get_user(user_id, ws_url) {
            // TODO: Add token expiration and re-login
            return Ok(Some(make_user(&huly_user)));
        }

        let login_info = login(&self.accounts_url, user_id, password).await;
        if let Err(err) = &login_info {
            tracing::error!("Error logging in: {:?}", err);
            // AuthenticationMiddleware can't handle errors, it crashes the request thread
            // Returning None will cause it to responce with 401 Unauthorized
            return Ok(None);
        }
        let login_info = login_info.unwrap();

        let mut huly_user = HulyUser {
            id: user_id.to_string(),
            contact_id: "".to_string(),
            social_id: login_info.social_id.clone(),
            account_uuid: login_info.account.clone(),
            workspace_url: "".to_string(),
            workspace_uuid: "".to_string(),
            token: login_info.token.clone(),
            api_url: "".to_string(),
        };

        let Some(ws_url) = ws_url else {
            let user = make_user(&huly_user);
            cache.set_user(user_id, ws_url, huly_user);
            return Ok(Some(user));
        };

        let user_workspaces = get_workspaces(&self.accounts_url, &login_info.token).await;
        if let Err(err) = &user_workspaces {
            tracing::error!("Error getting user workspaces: {:?}", err);
            return Ok(None);
        }
        let user_workspaces = user_workspaces.unwrap();
        let ws_uuid = user_workspaces
            .into_iter()
            .find(|(url, _)| url == ws_url)
            .map(|(_, uuid)| uuid);
        if ws_uuid.is_none() {
            tracing::error!("Error finding workspace uuid");
            return Ok(None);
        }

        let selected_ws = select_workspace(&self.accounts_url, &login_info.token, ws_url).await;
        if let Err(err) = &selected_ws {
            tracing::error!("Error selecting workspace: {:?}", err);
            // AuthenticationMiddleware can't handle errors, it crashes the request thread
            // Returning None will cause it to responce with 401 Unauthorized
            return Ok(None);
        }
        let selected_ws = selected_ws.unwrap();
        huly_user.workspace_url = ws_url.to_string();
        huly_user.workspace_uuid = ws_uuid.unwrap();
        huly_user.token = selected_ws.0;
        huly_user.api_url = selected_ws.1;

        let params = FindParams {
            class: CLASS_PERSON,
            query: HashMap::from([("personUuid", login_info.account.as_str())]),
            options: Some(FindOptions {
                projection: Some(HashMap::from([
                    ("_id", 1),
                ])),
            }),
        };
        let persons = find_all::<HulyPerson>(&huly_user, &params).await?;
        if persons.is_empty() {
            tracing::error!("Error finding local person for account {}", login_info.account);
            return Ok(None);
        }
        let person = persons.get(0).unwrap();
        huly_user.contact_id = person.id.clone();

        tracing::debug!("HULY_LOGIN {:?}", huly_user);

        let user = make_user(&huly_user);
        cache.set_user(user_id, Some(ws_url), huly_user);
        Ok(Some(user))
    }

    async fn get_principal(&self, principal: &str) -> Result<Option<User>, Error> {
        let cache = self.calendar_cache.lock().await;
        if let Some(huly_user) = cache.try_get_user(principal, None) {
            return Ok(Some(make_user(&huly_user)));
        }
        Ok(None)
    }

    async fn add_app_token(
        &self,
        _user_id: &str,
        _name: String,
        _token: String,
    ) -> Result<(), Error> {
        Ok(())
    }
}
