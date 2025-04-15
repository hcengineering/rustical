use jsonwebtoken::{encode, EncodingKey, Header};
use rustical_store::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) type PersonId = String;
pub(crate) type PersonUuid = String;
pub(crate) type WorkspaceUuid = String;

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct AccountRequestParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    social_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    social_key: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct AccountRequestIntegrationSecretParams<'a> {
    social_id: &'a str,
    kind: &'static str,
    key: &'static str,
    workspace_uuid: Option<&'a str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountRequest<TParams>
where
    TParams: Serialize,
{
    method: &'static str,
    params: TParams,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountError {
    code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountsResponce<TResult> {
    result: Option<TResult>,
    error: Option<AccountError>,
}

pub(crate) struct AccountClient<'a> {
    url: &'a str,
    token: String,
}

impl<'a> AccountClient<'a> {
    pub(crate) async fn new_with_account(
        url: &'a str,
        account_uuid: &str,
        secret: &str,
    ) -> Result<Self, Error> {
        let claims = TokenClaims {
            account: account_uuid,
            extra: Some(HashMap::from([("service", "caldav")])),
            ..Default::default()
        };
        let token = generate_token(&claims, secret)?;

        Ok(Self { url, token })
    }

    pub(crate) async fn new_with_token(url: &'a str, token: &str) -> Result<Self, Error> {
        Ok(Self {
            url,
            token: token.to_string(),
        })
    }

    async fn call<TParams, TResult>(
        &self,
        method: &'static str,
        params: TParams,
    ) -> Result<TResult, Error>
    where
        TParams: Serialize,
        TResult: for<'de> Deserialize<'de>,
    {
        account_post(self.url, &self.token, method, params).await
    }

    pub(crate) async fn find_social_id(&self, user_id: &str) -> Result<PersonId, Error> {
        let social_key = format!("huly:{}", user_id);
        self.call::<AccountRequestParams, PersonId>(
            "findSocialIdBySocialKey",
            AccountRequestParams {
                social_key: Some(&social_key),
                ..Default::default()
            },
        )
        .await
    }

    pub(crate) async fn find_account_uuid(&self, social_id: &str) -> Result<PersonUuid, Error> {
        self.call::<AccountRequestParams, PersonUuid>(
            "findPersonBySocialId",
            AccountRequestParams {
                social_id: Some(social_id),
                ..Default::default()
            },
        )
        .await
    }

    pub(crate) async fn list_integrations(
        &self,
        social_id: &str,
    ) -> Result<Vec<Integration>, Error> {
        self.call::<AccountRequestParams, Vec<Integration>>(
            "listIntegrations",
            AccountRequestParams {
                social_id: Some(social_id),
                kind: Some("caldav"),
                ..Default::default()
            },
        )
        .await
    }

    pub(crate) async fn get_integration_secret(
        &self,
        social_id: &str,
    ) -> Result<IntegrationSecret, Error> {
        self.call::<AccountRequestIntegrationSecretParams, IntegrationSecret>(
            "getIntegrationSecret",
            AccountRequestIntegrationSecretParams {
                social_id,
                kind: "caldav",
                key: "caldav",
                ..Default::default()
            },
        )
        .await
    }

    pub(crate) async fn get_workspaces_info(
        &self,
        workspaces: &[String],
    ) -> Result<Vec<WorkspaceInfoWithStatus>, Error> {
        self.call::<AccountRequestParams, Vec<WorkspaceInfoWithStatus>>(
            "getWorkspacesInfo",
            AccountRequestParams {
                ids: Some(workspaces.to_vec()),
                ..Default::default()
            },
        )
        .await
    }

    pub(crate) async fn select_workspace(
        &self,
        workspace_url: &str,
    ) -> Result<(String, String), Error> {
        let res = self
            .call::<AccountRequestParams, WorkspaceLoginInfo>(
                "selectWorkspace",
                AccountRequestParams {
                    workspace_url: Some(workspace_url),
                    kind: Some("external"),
                    ..Default::default()
                },
            )
            .await?;
        Ok((res.token, extract_api_url(&res.endpoint)))
    }
}

async fn account_post<'a, TParams, TResult>(
    url: &str,
    token: &str,
    method: &'static str,
    params: TParams,
) -> Result<TResult, Error>
where
    TParams: Serialize,
    TResult: for<'de> Deserialize<'de>,
{
    let req = AccountRequest { method, params };
    let client = reqwest::Client::new();
    let request = client
        .post(url)
        .json(&req)
        .header("Authorization", format!("Bearer {}", token));
    let response = request.send().await;
    if let Err(err) = response {
        return Err(Error::ApiError(format!(
            "Account servce {}: {:?}",
            req.method, err
        )));
    }
    let response = response.unwrap();
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await;
        if let Err(err) = text {
            //println!("*** ACCOUNT: resp_error: {:?}", err);
            return Err(Error::ApiError(format!(
                "Account servce {}: resp_error {}: {:?}",
                req.method, status, err
            )));
        }
        let text = text.unwrap();
        //println!("*** ACCUNT: status_error: {:?}", text);
        return Err(Error::ApiError(format!(
            "Account servce {}: status_error {}: {:?}",
            req.method, status, text
        )));
    }
    let text = response.text().await;
    if let Err(err) = text {
        //println!("*** ACCOUNT: resp_error: {:?}", err);
        return Err(Error::ApiError(format!(
            "Account servce {}: resp_error: {:?}",
            req.method, err
        )));
    }
    let text = text.unwrap();
    //println!("*** ACCOUNT: json_text: {:?}", text);
    let result = serde_json::from_str::<AccountsResponce<TResult>>(&text);
    if let Err(err) = &result {
        //println!("*** ACCOUNT: json_error: {:?}", err);
        return Err(Error::ApiError(format!(
            "Account servce {}: json_error: {:?}",
            req.method, err
        )));
    }
    let result = result.unwrap();
    if let Some(error) = result.error {
        return Err(Error::ApiError(format!(
            "Account servce {}: {:?}",
            req.method, error.code
        )));
    }
    if let Some(result) = result.result {
        return Ok(result);
    }
    Err(Error::ApiError(format!(
        "Account servce {}: empty result",
        req.method,
    )))
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct TokenClaims<'a> {
    pub(crate) account: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) workspace: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) extra: Option<HashMap<&'a str, &'a str>>,
}

pub(crate) fn generate_token(claims: &TokenClaims, secret: &str) -> Result<String, Error> {
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    );
    if let Err(err) = &token {
        return Err(Error::ApiError(format!(
            "Error generating token: {:?}",
            err
        )));
    }
    let token = token.unwrap();
    Ok(token)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceLoginInfo {
    pub(crate) token: String,
    pub(crate) endpoint: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceStatus {
    pub(crate) is_disabled: bool,
    pub(crate) mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceInfoWithStatus {
    pub(crate) uuid: WorkspaceUuid,
    pub(crate) url: String,
    pub(crate) status: WorkspaceStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Integration {
    pub(crate) workspace_uuid: Option<WorkspaceUuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationSecret {
    pub(crate) secret: String,
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
